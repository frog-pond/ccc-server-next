#![deny(clippy::all)]
#![warn(clippy::cargo, clippy::pedantic, clippy::cognitive_complexity)]

use axum::{
	error_handling::HandleErrorLayer, http::StatusCode, response::IntoResponse, routing::get,
	BoxError, Router, Server,
};
use tower::{timeout::TimeoutLayer, ServiceBuilder};

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
		.nest("/tools", ccc_routes::tools::router())
		.nest("/transit", ccc_routes::transit::router())
		.nest("/webcams", ccc_routes::webcams::router());

	Router::new()
		.nest("/", meta_routes)
		.nest("/api", api_routes)
		.layer(middleware_stack)
		.fallback(fallback)
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();

	let app = init_router();

	Server::bind(&"0.0.0.0:3000".parse().unwrap())
		.serve(app.into_make_service())
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
