//! Native entry point for running the CCC API as an HTTP server.

// The draft exposes the signatures for review before wiring up the server.
#![allow(dead_code, unused_variables)]
#![allow(clippy::todo, reason = "Signatures are under review in draft PR #487")]

use std::{
	fmt,
	future::Ready,
	io,
	net::{AddrParseError, SocketAddr},
	time::SystemTime,
};

use axum::Router;
use reqwest::Client;

/// Process settings parsed by the native entry point before starting the listener.
struct Config {
	/// IP address and port at which the native process accepts HTTP connections.
	listen_address: SocketAddr,
}

/// Provides the current wall-clock time to native handlers. A handler that
/// needs time passes the sampled timestamp to shared library code.
type Clock = fn() -> SystemTime;

/// Why the native process could not start or continue serving HTTP.
#[derive(Debug)]
enum ServerError {
	/// The listen address setting is present and is not Unicode.
	ListenAddressEncoding,
	/// The listen address setting is not an IP socket address.
	ListenAddressSyntax(AddrParseError),
	/// The Tokio runtime could not be initialized.
	Runtime(io::Error),
	/// The upstream HTTP client could not be initialized.
	HttpClient(reqwest::Error),
	/// The TCP listener could not bind to the configured address.
	Bind {
		/// The configured IP address and port the listener tried to bind.
		address: SocketAddr,
		/// The underlying failure returned by the TCP listener.
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
fn config_from_env() -> Result<Config, ServerError> {
	todo!("review draft")
}

/// Builds the native client for upstream requests. It returns redirects to the
/// caller without following them.
///
/// # Errors
///
/// Returns an error if reqwest cannot initialize the client.
fn build_http_client() -> Result<Client, ServerError> {
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

/// Runs the native HTTP server with the dependencies assembled by the entry point.
/// The listener uses the configured address. Handlers can use the supplied
/// clock and client when a route needs a timestamp or an upstream response.
///
/// # Errors
///
/// Returns an error if the listener cannot bind or Axum reports a server error.
async fn serve(config: Config, clock: Clock, http_client: Client) -> Result<(), ServerError> {
	todo!("review draft")
}

/// Assembles the configuration, clock, HTTP client, and Tokio runtime, then
/// runs the server with those dependencies.
///
/// # Errors
///
/// Returns configuration, client initialization, runtime initialization, listener,
/// or server errors.
fn main() -> Result<(), ServerError> {
	todo!("review draft")
}
