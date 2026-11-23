//! Native entry point for running the CCC API as an HTTP server.

#![allow(
	dead_code,
	unused_variables,
	reason = "Draft signatures are not wired together yet"
)]
#![allow(clippy::todo, reason = "Signatures are under review in draft PR #487")]

use std::{
	ffi::OsStr,
	fmt,
	future::{Future, Ready},
	io,
	net::{AddrParseError, SocketAddr},
	process::ExitCode,
	time::SystemTime,
};

use axum::{
	extract::Request,
	http::{Method, StatusCode},
	middleware::Next,
	response::Response,
	Json, Router,
};
use ccc_schema::error::RouteErrorBody;
use reqwest::Client;
use tokio::{net::TcpListener, runtime::Runtime};

/// The native HTTP client identifies itself by the binary crate's name and version.
const UPSTREAM_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Process settings parsed by the native entry point before starting the listener.
struct Config {
	/// Every IP address and port at which the process accepts HTTP connections.
	listen_addresses: NonEmpty<SocketAddr>,
}

/// One or more items, maintaining the first item through parsing and binding.
struct NonEmpty<T> {
	/// The required first item.
	first: T,
	/// Any additional items in their original order.
	rest: Vec<T>,
}

/// The server's Tokio runtime, constructed with I/O and time drivers enabled.
struct ServerRuntime(Runtime);

/// Provides the current wall-clock time to native handlers. A handler that
/// needs time passes the sampled timestamp to shared library code.
type Clock = fn() -> SystemTime;

/// Why the native process could not start serving HTTP.
#[derive(Debug)]
enum ServerError {
	/// The listen address setting is present and is not Unicode.
	ListenAddressEncoding,
	/// The listen address setting is not an IP socket address.
	ListenAddressSyntax {
		/// The list entry that could not be parsed.
		value: String,
		/// The socket-address parser's failure.
		source: AddrParseError,
	},
	/// The Tokio runtime could not be initialized.
	Runtime(io::Error),
	/// The upstream HTTP client could not be initialized.
	HttpClient(reqwest::Error),
	/// The process could not register a SIGTERM or SIGINT handler.
	SignalHandler(io::Error),
	/// The TCP listener could not bind to the configured address.
	Bind {
		/// The configured IP address and port the listener tried to bind.
		address: SocketAddr,
		/// The underlying failure returned by the TCP listener.
		source: io::Error,
	},
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

/// Parses the value of `CCC_LISTEN_ADDR` supplied by the entry point.
/// A comma-separated list accepts surrounding whitespace per entry.
/// An unset or blank value selects both `0.0.0.0:8080` and `[::]:8080`.
///
/// # Errors
///
/// Returns an error if the setting is not Unicode or any entry is not an IP
/// address with a port.
fn parse_config(value: Option<&OsStr>) -> Result<Config, ServerError> {
	todo!("review draft")
}

/// Builds the native client with the upstream User-Agent set to the crate name
/// and version. The client returns redirects without following them.
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

/// Returns the JSON 405 response for a method the native router does not serve.
fn method_not_allowed() -> Ready<(StatusCode, Json<RouteErrorBody>)> {
	todo!("review draft")
}

/// Returns the JSON 404 response for an unmatched GET. Other methods receive
/// the same JSON 405 response as they would on a matched path.
fn unmatched_path(method: Method) -> Ready<(StatusCode, Json<RouteErrorBody>)> {
	todo!("review draft")
}

/// Removes Axum's Allow header from every 405 to match the reference Worker.
async fn strip_allow_on_405(request: Request, next: Next) -> Response {
	todo!("review draft")
}

/// Builds the native router with the supplied clock and HTTP client available
/// for routes that need them. `/ping` accepts GET on every host. HEAD is
/// explicitly routed to the method error handler so Axum cannot use GET for it.
fn router(clock: Clock, http_client: Client) -> Router {
	todo!("review draft")
}

/// Builds the Tokio runtime with I/O and time drivers enabled.
///
/// # Errors
/// Returns an error if Tokio cannot initialize the runtime.
fn build_runtime() -> Result<ServerRuntime, ServerError> {
	todo!("review draft")
}

/// Binds every configured address and registers each listener on the supplied
/// runtime. The sockets use `SO_REUSEADDR` and a requested backlog of 1024.
/// IPv6 sockets accept only IPv6 connections so both families can share a port.
///
/// # Errors
///
/// Returns the attempted address and I/O error if any listener cannot be opened.
fn bind_listeners(
	runtime: &ServerRuntime,
	config: &Config,
) -> Result<NonEmpty<TcpListener>, ServerError> {
	todo!("review draft")
}

/// Registers SIGTERM and SIGINT on the supplied runtime. The returned future
/// completes when the first of the two signals arrives.
///
/// # Errors
///
/// Returns an error if the process cannot register the signal handler.
fn install_shutdown_signal(
	runtime: &ServerRuntime,
) -> Result<impl Future<Output = ()> + Send + 'static, ServerError> {
	Ok(async { todo!("review draft") })
}

/// Serves the same router on every bound listener on the supplied runtime.
/// One task awaits the shutdown future and broadcasts completion through
/// a Tokio watch channel. Every listener stops accepting and drains its requests.
///
/// # Panics
/// Calls Tokio's blocking entry point and must be invoked outside a running runtime.
fn serve(
	runtime: &ServerRuntime,
	listeners: NonEmpty<TcpListener>,
	router: Router,
	shutdown: impl Future<Output = ()> + Send + 'static,
) {
	todo!("review draft")
}

/// Initializes native dependencies, binds the listeners and serves until shutdown.
///
/// # Errors
///
/// Returns errors from runtime, client, listener or signal initialization.
fn run(config: Config, clock: Clock) -> Result<(), ServerError> {
	todo!("review draft")
}

/// Reads process configuration and starts the native server. On startup
/// failure, writes the error and its causes to stderr and returns failure.
fn main() -> ExitCode {
	todo!("review draft")
}
