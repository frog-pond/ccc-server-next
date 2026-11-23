//! TCP listeners and graceful shutdown for the native entry point.

use std::{io, net::SocketAddr, time::Duration};

use axum::{serve::Listener, Router};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{
	net::{TcpListener, TcpStream},
	signal::unix::{signal, SignalKind},
	sync::watch,
	task::JoinSet,
};

use crate::{Config, NonEmpty, ServerError};

/// A listening socket and the actual address used to identify it in events.
struct LoggingListener {
	socket: TcpListener,
	address: SocketAddr,
}

impl Listener for LoggingListener {
	type Io = TcpStream;
	type Addr = SocketAddr;

	async fn accept(&mut self) -> (Self::Io, Self::Addr) {
		loop {
			match self.socket.accept().await {
				Ok(connection) => return connection,
				Err(error)
					if matches!(
						error.kind(),
						io::ErrorKind::ConnectionRefused
							| io::ErrorKind::ConnectionAborted
							| io::ErrorKind::ConnectionReset
					) =>
				{
					tracing::debug!(address = %self.address, %error, "Connection closed before accept");
				}
				Err(error) => {
					tracing::warn!(
						name: "ccc.native.accept_failed",
						event = "ccc.native.accept_failed",
						address = %self.address,
						os_error = error.raw_os_error(),
						error = %error,
						"HTTP listener could not accept a connection",
					);
					tokio::time::sleep(Duration::from_secs(1)).await;
				}
			}
		}
	}

	fn local_addr(&self) -> io::Result<Self::Addr> {
		Ok(self.address)
	}
}

/// Opens a nonblocking socket with address reuse and a backlog of 1024.
/// An IPv6 socket accepts only IPv6 so both families can share a port.
///
/// # Errors
/// Returns an error if creating, configuring, binding, listening on, or
/// registering the socket fails.
fn bind_socket(address: SocketAddr) -> io::Result<LoggingListener> {
	let socket = Socket::new(
		Domain::for_address(address),
		Type::STREAM,
		Some(Protocol::TCP),
	)?;
	socket.set_reuse_address(true)?;
	if address.is_ipv6() {
		socket.set_only_v6(true)?;
	}
	socket.set_nonblocking(true)?;
	socket.bind(&address.into())?;
	socket.listen(1024)?;
	let socket = TcpListener::from_std(socket.into())?;
	Ok(LoggingListener {
		address: socket.local_addr()?,
		socket,
	})
}

/// Binds the whole list before any listener starts serving.
///
/// # Errors
/// Returns the configured address and the cause if any bind fails.
fn bind_listeners(config: Config) -> Result<NonEmpty<LoggingListener>, ServerError> {
	let bind = |address| bind_socket(address).map_err(|source| ServerError::Bind { address, source });
	Ok(NonEmpty {
		first: bind(config.listen_addresses.first)?,
		rest: config
			.listen_addresses
			.rest
			.into_iter()
			.map(bind)
			.collect::<Result<_, _>>()?,
	})
}

/// Waits for all listener tasks to finish.
///
/// # Errors
/// Returns the first listener failure or task join error.
async fn wait_for_listeners(
	tasks: &mut JoinSet<Result<(), ServerError>>,
) -> Result<(), ServerError> {
	while let Some(result) = tasks.join_next().await {
		result.map_err(ServerError::ListenerTask)??;
	}
	Ok(())
}

/// Serves one address until the shared shutdown notification arrives.
///
/// # Errors
/// Returns an error if Axum reports an I/O failure or stops before shutdown.
async fn serve_listener(
	listener: LoggingListener,
	router: Router,
	stopping: watch::Receiver<bool>,
) -> Result<(), ServerError> {
	let address = listener.address;
	let mut notification = stopping.clone();
	axum::serve(listener, router)
		.with_graceful_shutdown(async move {
			let _ = notification.wait_for(|stop| *stop).await;
		})
		.await
		.map_err(|source| ServerError::ListenerIo { address, source })?;
	if *stopping.borrow() {
		Ok(())
	} else {
		Err(ServerError::ListenerStopped(address))
	}
}

/// Starts one copy of the router per listener. A first signal triggers graceful
/// shutdown. A second signal or an eight-second deadline ends the wait.
///
/// # Errors
/// Returns an error if a listener or its task fails.
async fn serve(
	listeners: NonEmpty<LoggingListener>,
	router: Router,
	mut shutdown: impl AsyncFnMut(),
) -> Result<(), ServerError> {
	let (stop, _) = watch::channel(false);
	let mut tasks = JoinSet::new();
	for listener in std::iter::once(listeners.first).chain(listeners.rest) {
		tasks.spawn(serve_listener(listener, router.clone(), stop.subscribe()));
	}
	tokio::select! {
		result = wait_for_listeners(&mut tasks) => return result,
		() = shutdown() => { stop.send_replace(true); }
	}
	tokio::select! {
		biased;
		() = shutdown() => {
			tracing::info!(name: "ccc.native.shutdown_forced", event = "ccc.native.shutdown_forced", "Received a second shutdown signal");
			Ok(())
		}
		result = wait_for_listeners(&mut tasks) => result,
		() = tokio::time::sleep(Duration::from_secs(8)) => {
			tracing::warn!(name: "ccc.native.shutdown_timeout", event = "ccc.native.shutdown_timeout", "Graceful shutdown reached the eight-second limit");
			Ok(())
		}
	}
}

/// Runs inside the entry point's I/O-enabled runtime, including socket and
/// signal registration.
///
/// # Errors
/// Returns the signal, bind, or listener failure that stopped the server.
pub(super) async fn run(config: Config, router: Router) -> Result<(), ServerError> {
	let mut terminate = signal(SignalKind::terminate()).map_err(ServerError::SignalHandler)?;
	let mut interrupt = signal(SignalKind::interrupt()).map_err(ServerError::SignalHandler)?;
	let listeners = bind_listeners(config)?;
	serve(listeners, router, async || {
		tokio::select! { _ = terminate.recv() => {}, _ = interrupt.recv() => {} }
	})
	.await
}

#[cfg(test)]
mod tests {
	use std::{sync::Arc, time::Duration};

	use axum::{routing::get, Router};
	use tokio::{
		net::TcpStream,
		sync::{mpsc, oneshot},
		task::{spawn_local, JoinHandle, LocalSet},
		time::timeout,
	};

	use super::{bind_socket, serve, NonEmpty, ServerError};

	struct InFlight {
		signal: mpsc::Sender<()>,
		release: oneshot::Sender<()>,
		server: JoinHandle<Result<(), ServerError>>,
		response: JoinHandle<reqwest::Response>,
	}

	async fn start_request() -> InFlight {
		let listener = bind_socket("127.0.0.1:0".parse().expect("address")).expect("listener");
		let address = listener.address;
		let (entered, handler_started) = oneshot::channel();
		let (release, resume) = oneshot::channel();
		let pending = Arc::new(tokio::sync::Mutex::new(Some((entered, resume))));
		let router = Router::new().route(
			"/",
			get(move || {
				let pending = Arc::clone(&pending);
				async move {
					let (entered, resume) = pending.lock().await.take().expect("one request");
					entered.send(()).expect("request observer");
					resume.await.expect("request released");
					"completed"
				}
			}),
		);
		let (signal, mut signals) = mpsc::channel(2);
		let server = spawn_local(async move {
			serve(
				NonEmpty {
					first: listener,
					rest: Vec::new(),
				},
				router,
				async || {
					signals.recv().await.expect("shutdown signal");
				},
			)
			.await
		});
		let response = tokio::spawn(async move {
			reqwest::Client::builder()
				.no_proxy()
				.build()
				.expect("client")
				.get(format!("http://{address}/"))
				.send()
				.await
				.expect("response")
		});
		timeout(Duration::from_secs(2), handler_started)
			.await
			.expect("handler entered")
			.expect("handler notification");
		signal.send(()).await.expect("first shutdown signal");
		timeout(Duration::from_secs(2), async {
			while TcpStream::connect(address).await.is_ok() {
				tokio::time::sleep(Duration::from_millis(5)).await;
			}
		})
		.await
		.expect("listener stopped accepting");
		InFlight {
			signal,
			release,
			server,
			response,
		}
	}

	#[tokio::test]
	async fn first_signal_finishes_requests_in_flight() {
		LocalSet::new()
			.run_until(async {
				let pending = start_request().await;
				assert!(!pending.server.is_finished());
				pending.release.send(()).expect("release handler");
				let response = timeout(Duration::from_secs(2), pending.response)
					.await
					.expect("response deadline")
					.expect("response task");
				assert_eq!(response.text().await.expect("body"), "completed");
				pending
					.server
					.await
					.expect("server task")
					.expect("clean exit");
			})
			.await;
	}

	#[tokio::test]
	async fn second_signal_ends_the_wait_for_a_running_request() {
		LocalSet::new()
			.run_until(async {
				let pending = start_request().await;
				assert!(!pending.server.is_finished());
				pending
					.signal
					.send(())
					.await
					.expect("second shutdown signal");
				timeout(Duration::from_secs(2), pending.server)
					.await
					.expect("forced exit deadline")
					.expect("server task")
					.expect("clean exit");
				pending.response.abort();
			})
			.await;
	}

	#[tokio::test]
	async fn shutdown_deadline_ends_the_wait_for_a_running_request() {
		LocalSet::new()
			.run_until(async {
				let mut pending = start_request().await;
				assert!(timeout(Duration::from_secs(1), &mut pending.server)
					.await
					.is_err());
				timeout(Duration::from_secs(8), pending.server)
					.await
					.expect("shutdown deadline")
					.expect("server task")
					.expect("clean exit");
				pending.response.abort();
			})
			.await;
	}
}
