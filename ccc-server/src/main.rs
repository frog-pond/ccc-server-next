#![deny(clippy::all)]
#![warn(clippy::cargo, clippy::pedantic, clippy::cognitive_complexity)]

use axum::{
	error_handling::HandleErrorLayer, http::StatusCode, response::IntoResponse, routing::get,
	BoxError, Router,
};
use clap::{Parser, ValueEnum};
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tracing_subscriber::{
	fmt::{format, layer},
	layer::SubscriberExt,
	util::SubscriberInitExt,
	Layer,
};

#[derive(Clone, Debug, PartialEq, ValueEnum)]
enum LogStructure {
	Default,
	Debug,
	Json,
	Pretty,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Tracing format
	#[clap(value_enum, short, long, default_value_t=LogStructure::Default)]
	tracing: LogStructure,
}

fn init_router() -> Router {
	let middleware_stack = ServiceBuilder::new()
		.layer(tower_http::trace::TraceLayer::new_for_http())
		.layer(HandleErrorLayer::new(error_handler))
		.layer(TimeoutLayer::new(core::time::Duration::from_secs(10)));

	let meta_routes = Router::new()
		.route("/", get(root_handler))
		.route("/ping", get(heartbeat_handler));

	let api_routes = Router::new()
		.nest("/contacts", ccc_routes::contacts::router())
		.nest("/dictionary", ccc_routes::dictionary::router())
		.nest("/faqs", ccc_routes::faqs::router())
		.nest("/food", ccc_routes::food::router())
		.nest("/printing", ccc_routes::printing::router())
		.nest("/spaces", ccc_routes::spaces::router())
		.nest("/streams", ccc_routes::streams::router())
		.nest("/tools", ccc_routes::tools::router())
		.nest("/transit", ccc_routes::transit::router())
		.nest("/webcams", ccc_routes::webcams::router());

	Router::new()
		.merge(meta_routes)
		.nest("/api", api_routes)
		.layer(middleware_stack)
		.fallback(fallback)
}

#[tokio::main]
async fn main() {
	let args = Args::parse();

	init_tracing(args.tracing);

	let app = init_router();

	let tcp_listener =
		tokio::net::TcpListener::bind("0.0.0.0:3000".parse::<std::net::SocketAddr>().unwrap())
			.await
			.expect("failed to bind");

	axum::serve(tcp_listener, app.into_make_service())
		.await
		.expect("server failed to exit successfully");
}

#[allow(clippy::unused_async)]
async fn error_handler(error: BoxError) -> impl IntoResponse {
	if error.is::<tower::timeout::error::Elapsed>() {
		(
			StatusCode::REQUEST_TIMEOUT,
			"Request took too long".to_string(),
		)
	} else {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Unhandled Internal Error: {error}"),
		)
	}
}

fn init_tracing(tracing: LogStructure) {
	let output = match tracing {
		LogStructure::Default => layer().boxed(), // ideally unreachable but would otherwise error
		LogStructure::Debug => layer().fmt_fields(format::Pretty::default()).boxed(),
		LogStructure::Json => layer().json().boxed(),
		LogStructure::Pretty => layer()
			.event_format(format::Format::default().with_source_location(false))
			.fmt_fields(format::PrettyFields::new())
			.with_target(false)
			.boxed(),
	};

	let env_filter = tracing_subscriber::EnvFilter::from_default_env();

	if tracing == LogStructure::Default {
		// prefer fmt+init to retain a compact ouput whereas layer+boxed is overly verbose
		tracing_subscriber::fmt().with_env_filter(env_filter).init();
	} else {
		tracing_subscriber::registry()
			.with(env_filter)
			.with(output)
			.init();
	}
}

#[allow(clippy::unused_async)]
async fn root_handler() -> Result<&'static str, StatusCode> {
	Ok("Hello world!")
}

#[allow(clippy::unused_async)]
async fn heartbeat_handler() -> Result<&'static str, StatusCode> {
	Ok("pong")
}

#[allow(clippy::unused_async)]
async fn fallback() -> impl IntoResponse {
	(StatusCode::NOT_FOUND, "Not Found".to_string())
}
