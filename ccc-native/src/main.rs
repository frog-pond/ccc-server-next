//! Native entry point for running the CCC API as an HTTP server.

mod healthcheck;
mod server;

use std::{
	error::Error,
	ffi::{OsStr, OsString},
	fmt,
	future::{ready, Ready},
	io::{self, Write},
	net::{AddrParseError, Ipv4Addr, Ipv6Addr, SocketAddr},
	process::ExitCode,
	time::{Duration, SystemTime},
};

use axum::{
	extract::Request,
	http::{Method, StatusCode, Uri},
	middleware::{self, Next},
	response::{IntoResponse, Response},
	routing::any,
	Json, Router,
};
use ccc_schema::routing::{RouteErrorBody, RouteErrorMessage};
use reqwest::{redirect::Policy, Client, Url};
use tower::Layer;

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

/// The two commands supported by the native binary.
enum Mode {
	/// Start the HTTP server.
	Serve,
	/// Probe the configured listeners and exit.
	Healthcheck,
}

/// Provides a sampled wall-clock timestamp to a route when it needs one.
type Clock = fn() -> SystemTime;

/// Why the native process could not start or continue serving HTTP.
#[derive(Debug)]
enum ServerError {
	/// The command line is neither empty nor the single healthcheck command.
	InvalidArguments,
	/// The health check cannot discover the port chosen by a different process.
	HealthcheckEphemeralPort(SocketAddr),
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
		/// The listener’s actual bound address.
		address: SocketAddr,
		/// The I/O error returned by Axum.
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
			Self::InvalidArguments => formatter.write_str("usage: ccc-native [healthcheck]"),
			Self::HealthcheckEphemeralPort(address) => write!(
				formatter,
				"healthcheck needs a fixed port, but {address} requests an ephemeral port"
			),
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
			Self::InvalidArguments
			| Self::HealthcheckEphemeralPort(_)
			| Self::ListenAddressEncoding
			| Self::ListenerStopped(_) => None,
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

/// Parses one address list entry, ignoring surrounding whitespace.
///
/// # Errors
/// Returns an error carrying the entry if it is not an IP socket address.
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

/// Normalizes the path before Axum selects a route, following the Worker’s
/// WHATWG URL parsing. The request’s authority and query remain intact.
async fn normalize_path(mut request: Request, next: Next) -> Response {
	if let Some(uri) = normalized_uri(request.uri()) {
		*request.uri_mut() = uri;
	}
	next.run(request).await
}

/// Returns a replacement URI only when URL parsing changes the request path.
fn normalized_uri(uri: &Uri) -> Option<Uri> {
	let path = uri.path();
	if !path.starts_with('/') {
		return None;
	}
	let parsed = Url::parse(&format!("http://ccc.invalid{path}")).ok()?;
	if parsed.path() == path {
		return None;
	}
	let path_and_query = uri.query().map_or_else(
		|| parsed.path().to_owned(),
		|query| format!("{}?{query}", parsed.path()),
	);
	let mut parts = uri.clone().into_parts();
	parts.path_and_query = Some(path_and_query.parse().ok()?);
	Uri::from_parts(parts).ok()
}

/// The health route is static. Future upstream routes can use the injected
/// clock and client when the router assembles them.
fn router(_clock: Clock, _http_client: Client) -> Router {
	let routes = Router::new()
		.route(
			"/ping",
			any(method_not_allowed).get(ping).head(method_not_allowed),
		)
		.fallback(unmatched_path);
	Router::new().fallback_service(middleware::from_fn(normalize_path).layer(routes))
}

/// Parses the optional command name.
///
/// # Errors
/// Returns an error if any command is unrecognized or takes extra arguments.
fn parse_mode(mut args: impl Iterator<Item = OsString>) -> Result<Mode, ServerError> {
	match args.next() {
		None => Ok(Mode::Serve),
		Some(command) if command == OsStr::new("healthcheck") && args.next().is_none() => {
			Ok(Mode::Healthcheck)
		}
		Some(_) => Err(ServerError::InvalidArguments),
	}
}

/// Runs the selected mode inside the process runtime.
///
/// # Errors
/// Returns `HealthcheckEphemeralPort` in health-check mode. In serve mode,
/// returns HTTP client initialization errors or signal, bind, and listener
/// errors from `server::run`.
async fn run(config: Config, mode: Mode, clock: Clock) -> Result<ExitCode, ServerError> {
	match mode {
		Mode::Serve => {
			server::run(config, router(clock, build_http_client()?)).await?;
			Ok(ExitCode::SUCCESS)
		}
		Mode::Healthcheck => Ok(if healthcheck::run(&config).await? {
			ExitCode::SUCCESS
		} else {
			ExitCode::FAILURE
		}),
	}
}

fn main() -> ExitCode {
	match start() {
		Ok(exit) => exit,
		Err(error) => {
			report_failure(&error);
			ExitCode::FAILURE
		}
	}
}

/// Formats the cause chain as one field without dropping intermediate errors.
struct ErrorCauses<'a>(&'a dyn Error);

impl fmt::Display for ErrorCauses<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut cause = self.0.source();
		let mut separator = "";
		while let Some(error) = cause {
			write!(formatter, "{separator}{error}")?;
			separator = ": ";
			cause = error.source();
		}
		Ok(())
	}
}

/// Records the failure and causes, falling back to stderr before tracing is installed.
fn report_failure(error: &ServerError) {
	if matches!(error, ServerError::Tracing(_)) {
		let mut stderr = io::stderr().lock();
		let _ = writeln!(stderr, "{error}");
		let mut cause = error.source();
		while let Some(error) = cause {
			let _ = writeln!(stderr, "  caused by {error}");
			cause = error.source();
		}
	} else {
		tracing::error!(
			name: "ccc.native.server_failed",
			event = "ccc.native.server_failed",
			error = %error,
			causes = %ErrorCauses(error),
			"Native process failed",
		);
	}
}

/// Installs process services and runs the selected native command.
///
/// # Errors
/// Returns tracing, command-line, configuration, runtime, or command errors.
#[allow(
	clippy::disallowed_methods,
	reason = "The entry point supplies the wall clock to the native router"
)]
fn start() -> Result<ExitCode, ServerError> {
	let subscriber = tracing_subscriber::fmt()
		.with_writer(io::stderr)
		// The formatter otherwise uses eprintln! to report a failed log write.
		.log_internal_errors(false)
		.finish();
	tracing::subscriber::set_global_default(subscriber).map_err(ServerError::Tracing)?;
	let mode = parse_mode(std::env::args_os().skip(1))?;
	let config = parse_config(std::env::var_os("CCC_LISTEN_ADDR").as_deref())?;
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.map_err(ServerError::Runtime)?;
	let result = runtime.block_on(run(config, mode, SystemTime::now));
	runtime.shutdown_background();
	result
}

#[cfg(test)]
mod routing_tests;

#[cfg(test)]
mod tests {
	use std::ffi::OsString;

	use super::{parse_mode, ServerError};

	#[test]
	fn healthcheck_rejects_extra_arguments() {
		let args = ["healthcheck", "x"].into_iter().map(OsString::from);
		assert!(matches!(
			parse_mode(args),
			Err(ServerError::InvalidArguments)
		));
	}
}
