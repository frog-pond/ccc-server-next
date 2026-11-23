//! Native entry point for running the CCC API as an HTTP server.

mod server;

use std::{
	error::Error,
	ffi::OsStr,
	fmt,
	future::{ready, Ready},
	io,
	net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
	process::ExitCode,
	time::{Duration, SystemTime},
};

use axum::{
	http::{Method, StatusCode},
	response::{IntoResponse, Response},
	routing::any,
	Json, Router,
};
use ccc_schema::routing::{RouteErrorBody, RouteErrorMessage};
use reqwest::{redirect::Policy, Client};

/// The native HTTP client identifies itself by the binary crate's name and version.
const UPSTREAM_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Process settings parsed by the native entry point.
struct Config {
	listen_addresses: NonEmpty<SocketAddr>,
}

/// One or more items, preserving the first item through parsing and binding.
struct NonEmpty<T> {
	first: T,
	rest: Vec<T>,
}

/// Provides a sampled wall-clock timestamp to a route when it needs one.
type Clock = fn() -> SystemTime;

/// Why the native process could not start or continue serving HTTP.
#[derive(Debug)]
enum ServerError {
	/// The address list cannot be decoded as Unicode.
	ListenAddressEncoding,
	/// One list entry is not an IP address with a port.
	ListenAddressSyntax {
		/// The entry supplied by the operator.
		value: String,
		/// The socket address parser's error.
		source: AddrParseError,
	},
	/// Tokio could not create the server runtime.
	Runtime(io::Error),
	/// Reqwest could not construct the upstream client.
	HttpClient(reqwest::Error),
	/// The process could not install its tracing subscriber.
	Tracing(tracing::subscriber::SetGlobalDefaultError),
	/// Tokio could not register SIGTERM or SIGINT.
	SignalHandler(io::Error),
	/// One configured listener could not be opened.
	Bind {
		/// The configured IP address and port the listener tried to bind.
		address: SocketAddr,
		/// The error from creating, binding, listening on, or registering the socket.
		source: io::Error,
	},
	/// The listener returned an unexpected I/O error while serving.
	ListenerIo {
		address: SocketAddr,
		source: io::Error,
	},
	/// A listener finished serving ahead of the shutdown signal.
	ListenerStopped(SocketAddr),
	/// Tokio could not join a spawned listener task.
	ListenerTask(tokio::task::JoinError),
}

impl fmt::Display for ServerError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::ListenAddressEncoding => formatter.write_str("CCC_LISTEN_ADDR is not valid Unicode"),
			Self::ListenAddressSyntax { value, .. } => {
				write!(formatter, "invalid CCC_LISTEN_ADDR entry {value:?}")
			}
			Self::Runtime(_) => formatter.write_str("could not initialize the Tokio runtime"),
			Self::HttpClient(_) => formatter.write_str("could not initialize the HTTP client"),
			Self::Tracing(_) => formatter.write_str("could not install the tracing subscriber"),
			Self::SignalHandler(_) => {
				formatter.write_str("could not install the shutdown signal handlers")
			}
			Self::Bind { address, .. } => write!(formatter, "could not bind HTTP listener at {address}"),
			Self::ListenerIo { address, .. } => write!(formatter, "HTTP listener at {address} failed"),
			Self::ListenerStopped(address) => write!(
				formatter,
				"HTTP listener at {address} stopped before shutdown"
			),
			Self::ListenerTask(_) => formatter.write_str("an HTTP listener task ended abnormally"),
		}
	}
}

impl Error for ServerError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::ListenAddressEncoding | Self::ListenerStopped(_) => None,
			Self::ListenAddressSyntax { source, .. } => Some(source),
			Self::Runtime(source)
			| Self::SignalHandler(source)
			| Self::Bind { source, .. }
			| Self::ListenerIo { source, .. } => Some(source),
			Self::HttpClient(source) => Some(source),
			Self::Tracing(source) => Some(source),
			Self::ListenerTask(source) => Some(source),
		}
	}
}

/// Parses the process setting provided by main. No other server component reads
/// listener configuration. Blank or absent values select both wildcards at 8080.
///
/// # Errors
/// Returns an error for non-Unicode values or an invalid socket address entry.
fn parse_config(value: Option<&OsStr>) -> Result<Config, ServerError> {
	let value = value
		.map(|value| value.to_str().ok_or(ServerError::ListenAddressEncoding))
		.transpose()?
		.unwrap_or_default();
	let listen_addresses = if value.trim().is_empty() {
		NonEmpty {
			first: SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080)),
			rest: vec![SocketAddr::from((Ipv6Addr::UNSPECIFIED, 8080))],
		}
	} else {
		let (first, rest) = value
			.split_once(',')
			.map_or((value, None), |(first, rest)| (first, Some(rest)));
		NonEmpty {
			first: parse_address(first)?,
			rest: rest
				.into_iter()
				.flat_map(|rest| rest.split(','))
				.map(parse_address)
				.collect::<Result<_, _>>()?,
		}
	};
	Ok(Config { listen_addresses })
}

fn parse_address(value: &str) -> Result<SocketAddr, ServerError> {
	let value = value.trim();
	value
		.parse()
		.map_err(|source| ServerError::ListenAddressSyntax {
			value: value.to_owned(),
			source,
		})
}

/// Builds the native client with a five-second connect timeout and a
/// thirty-second total timeout. It bypasses proxies and returns redirects
/// unchanged. Upstream requests carry the binary crate's name and version.
///
/// # Errors
/// Returns an error if reqwest cannot initialize the client.
fn build_http_client() -> Result<Client, ServerError> {
	Client::builder()
		.user_agent(UPSTREAM_USER_AGENT)
		.redirect(Policy::none())
		.no_proxy()
		.connect_timeout(Duration::from_secs(5))
		.timeout(Duration::from_secs(30))
		.build()
		.map_err(ServerError::HttpClient)
}

/// Maps the shared response message to its HTTP status at the native boundary.
struct RoutingFailure(RouteErrorMessage);

impl IntoResponse for RoutingFailure {
	fn into_response(self) -> Response {
		let status = match self.0 {
			RouteErrorMessage::NotFound => StatusCode::NOT_FOUND,
			RouteErrorMessage::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
		};
		(status, Json(RouteErrorBody { error: self.0 })).into_response()
	}
}

fn ping() -> Ready<&'static str> {
	ready("pong")
}

fn method_not_allowed() -> Ready<RoutingFailure> {
	ready(RoutingFailure(RouteErrorMessage::MethodNotAllowed))
}

#[allow(
	clippy::needless_pass_by_value,
	reason = "Axum extracts the request method by value"
)]
fn unmatched_path(method: Method) -> Ready<RoutingFailure> {
	ready(RoutingFailure(if method == Method::GET {
		RouteErrorMessage::NotFound
	} else {
		RouteErrorMessage::MethodNotAllowed
	}))
}

/// The health route is static. Future upstream routes can use the injected
/// clock and client when the router assembles them.
fn router(_clock: Clock, _http_client: Client) -> Router {
	Router::new()
		.route(
			"/ping",
			any(method_not_allowed).get(ping).head(method_not_allowed),
		)
		.fallback(unmatched_path)
}

async fn healthcheck(config: &Config, client: &Client) -> bool {
	for address in
		std::iter::once(&config.listen_addresses.first).chain(&config.listen_addresses.rest)
	{
		let check = async {
			let response = client.get(healthcheck_url(*address)).send().await?;
			Ok::<_, reqwest::Error>(
				response.status() == StatusCode::OK && response.bytes().await? == "pong",
			)
		};
		if matches!(
			tokio::time::timeout(Duration::from_millis(500), check).await,
			Ok(Ok(true))
		) {
			return true;
		}
	}
	false
}

fn healthcheck_url(mut address: SocketAddr) -> String {
	if address.ip().is_unspecified() {
		address.set_ip(if address.is_ipv4() {
			IpAddr::V4(Ipv4Addr::LOCALHOST)
		} else {
			IpAddr::V6(Ipv6Addr::LOCALHOST)
		});
	}
	format!("http://{address}/ping")
}

async fn run(config: Config, probe: bool, clock: Clock) -> Result<ExitCode, ServerError> {
	let client = build_http_client()?;
	if probe {
		let healthy = tokio::time::timeout(Duration::from_secs(1), healthcheck(&config, &client))
			.await
			.unwrap_or(false);
		return Ok(if healthy {
			ExitCode::SUCCESS
		} else {
			ExitCode::FAILURE
		});
	}
	server::run(config, router(clock, client)).await?;
	Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
	match start() {
		Ok(exit) => exit,
		Err(error) => {
			eprintln!("{error}");
			let mut cause = error.source();
			while let Some(error) = cause {
				eprintln!("  caused by {error}");
				cause = error.source();
			}
			ExitCode::FAILURE
		}
	}
}

#[allow(
	clippy::disallowed_methods,
	reason = "The entry point supplies the wall clock to the native router"
)]
fn start() -> Result<ExitCode, ServerError> {
	let subscriber = tracing_subscriber::fmt().with_writer(io::stderr).finish();
	tracing::subscriber::set_global_default(subscriber).map_err(ServerError::Tracing)?;
	let config = parse_config(std::env::var_os("CCC_LISTEN_ADDR").as_deref())?;
	let probe = std::env::args_os().nth(1).as_deref() == Some(OsStr::new("healthcheck"));
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.map_err(ServerError::Runtime)?;
	let result = runtime.block_on(run(config, probe, SystemTime::now));
	runtime.shutdown_background();
	result
}

#[cfg(test)]
mod routing_tests;
