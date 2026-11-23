//! Direct HTTP/1 health probes of the native server's configured sockets.

use std::{
	net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
	time::Duration,
};

use axum::http::{header, Request, StatusCode};
use http_body_util::{BodyExt, Empty, Limited};
use hyper::{body::Bytes, client::conn::http1};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpStream, time::timeout};

use crate::{Config, ServerError};

/// Checks whether a configured listener returns HTTP 200 and exactly `pong`.
///
/// # Errors
/// Returns an error if any configured listener uses port zero, since this
/// process cannot discover what port the running server was assigned.
pub(super) async fn run(config: &Config) -> Result<bool, ServerError> {
	let addresses =
		|| std::iter::once(&config.listen_addresses.first).chain(&config.listen_addresses.rest);
	if let Some(address) = addresses().find(|address| address.port() == 0) {
		return Err(ServerError::HealthcheckEphemeralPort(*address));
	}
	let checks = async {
		for address in addresses() {
			if matches!(
				timeout(Duration::from_millis(500), probe(*address)).await,
				Ok(Some(true))
			) {
				return true;
			}
		}
		false
	};
	Ok(
		timeout(Duration::from_secs(1), checks)
			.await
			.unwrap_or(false),
	)
}

async fn probe(mut address: SocketAddr) -> Option<bool> {
	if address.ip().is_unspecified() {
		address.set_ip(if address.is_ipv4() {
			IpAddr::V4(Ipv4Addr::LOCALHOST)
		} else {
			IpAddr::V6(Ipv6Addr::LOCALHOST)
		});
	}
	let stream = TcpStream::connect(address).await.ok()?;
	let (mut sender, connection) = http1::handshake(TokioIo::new(stream)).await.ok()?;
	let request = Request::builder()
		.uri("/ping")
		.header(header::HOST, "localhost")
		.header(header::CONNECTION, "close")
		.body(Empty::<Bytes>::new())
		.ok()?;
	let check = async move {
		let response = sender.send_request(request).await.ok()?;
		let status = response.status();
		let body = Limited::new(response.into_body(), 4)
			.collect()
			.await
			.ok()?
			.to_bytes();
		Some(status == StatusCode::OK && body == "pong")
	};
	let (_, healthy) = tokio::join!(connection, check);
	healthy
}

#[cfg(test)]
mod tests {
	use std::ffi::OsStr;

	use axum::{routing::get, Router};
	use tokio::net::TcpListener;

	use super::run;
	use crate::{parse_config, ServerError};

	#[tokio::test]
	async fn healthcheck_rejects_an_ephemeral_port() {
		let config = parse_config(Some(OsStr::new("127.0.0.1:8080,[::1]:0"))).expect("config");
		assert!(
			matches!(run(&config).await, Err(ServerError::HealthcheckEphemeralPort(address)) if address.port() == 0)
		);
	}

	#[tokio::test]
	async fn healthcheck_connects_to_a_scoped_ipv6_address() {
		let listener = TcpListener::bind("[::1]:0").await.expect("IPv6 loopback");
		let port = listener.local_addr().expect("address").port();
		let server = tokio::spawn(async move {
			axum::serve(
				listener,
				Router::new().route("/ping", get(|| async { "pong" })),
			)
			.await
			.expect("serve");
		});
		let setting = format!("[::1%1]:{port}");
		let config = parse_config(Some(OsStr::new(&setting))).expect("scoped address");
		let result = run(&config).await;
		server.abort();
		assert!(result.expect("scoped probe"));
	}
}
