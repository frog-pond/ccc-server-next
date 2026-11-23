//! Native entry point for running the CCC API as an HTTP server.

// The draft exposes the signatures for review before wiring up the server.
#![allow(dead_code, unused_variables)]

use std::{
	fmt,
	future::Ready,
	io,
	net::{AddrParseError, SocketAddr},
};

use axum::Router;

/// Why the native process could not start or continue serving HTTP.
#[derive(Debug)]
enum ServerError {
	/// The listen address setting is present and is not Unicode.
	ListenAddressEncoding,
	/// The listen address setting is not an IP socket address.
	ListenAddressSyntax(AddrParseError),
	/// The Tokio runtime could not be initialized.
	Runtime(io::Error),
	/// The TCP listener could not bind to the configured address.
	Bind {
		address: SocketAddr,
		source: io::Error,
	},
	/// The HTTP server terminated with an I/O error.
	Serve(io::Error),
}

impl fmt::Display for ServerError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!("review draft")
	}
}

impl std::error::Error for ServerError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		todo!("review draft")
	}
}

/// Reads `CCC_LISTEN_ADDR` as an IP address and port, defaulting to
/// `0.0.0.0:8080` when it is unset.
///
/// # Errors
///
/// Returns an error if the setting is not Unicode or has invalid socket syntax.
fn listen_address() -> Result<SocketAddr, ServerError> {
	todo!("review draft")
}

/// Returns the fixed text of the native process's root health check.
fn ping() -> Ready<&'static str> {
	todo!("review draft")
}

/// Maps native HTTP requests to the routes served by this process.
fn router() -> Router {
	todo!("review draft")
}

/// Binds the listener and serves the native HTTP routes.
///
/// # Errors
///
/// Returns an error if the listener cannot bind or Axum reports a server error.
async fn serve(address: SocketAddr) -> Result<(), ServerError> {
	todo!("review draft")
}

/// Reads the native configuration, builds the Tokio runtime and runs the server.
///
/// # Errors
///
/// Returns configuration, runtime initialization, listener, or server errors.
fn main() -> Result<(), ServerError> {
	todo!("review draft")
}
