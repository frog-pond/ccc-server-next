use axum::{
	body::Body,
	error_handling::HandleErrorLayer,
	handler::Handler,
	http::{uri, Request, StatusCode},
	response::{IntoResponse, Response},
	routing::get,
	BoxError, Router, Server,
};
use std::convert::Infallible;
use tower::{filter::AsyncFilterLayer, util::AndThenLayer, ServiceBuilder};

fn init_router() -> Router {
	let middleware_stack = ServiceBuilder::new()
		.layer(HandleErrorLayer::new(|error| async move {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("Unhandled Internal Error: {}", error),
			)
		}))
		.layer(AsyncFilterLayer::new(map_request))
		.layer(AndThenLayer::new(map_response));

	let meta_routes = Router::new()
		.route("/", get(root_handler))
		.route("/ping", get(heartbeat_handler));

	let api_routes = Router::new()
		.nest("/contacts", routes::contacts::router())
		.nest("/dictionary", routes::dictionary::router())
		.nest("/faqs", routes::faqs::router())
		.nest("/food", routes::food::router())
		.nest("/printing", routes::printing::router())
		.nest("/spaces", routes::spaces::router())
		.nest("/tools", routes::tools::router())
		.nest("/transit", routes::transit::router())
		.nest("/webcams", routes::webcams::router());

	Router::new()
		.nest("/", meta_routes)
		.nest("/api", api_routes)
		.layer(middleware_stack)
		.fallback(fallback.into_service())
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
async fn map_request(req: Request<Body>) -> Result<Request<Body>, BoxError> {
	let (mut parts, body) = req.into_parts();

	let new_path = parts.uri.path().replace("/v1", "");
	let uri = uri::Builder::new().path_and_query(new_path).build()?;

	parts.uri = uri;

	Ok(Request::from_parts(parts, body))
}

#[allow(clippy::unused_async)]
async fn map_response(res: Response) -> Result<Response, Infallible> {
	Ok(res)
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
