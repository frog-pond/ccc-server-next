//! Native entry point for the local fixture route in #452. Tokio drives the
//! listener and Axum calls the handler, which runs the shared Trifecta operation.

// These signatures are under review before the native bodies are written.
#![allow(dead_code)]

use std::sync::Arc;

use axum::{
	extract::{Path, State},
	response::Response,
	routing::get,
	Router,
};
use fixture_experiment_route::{buffered_fixture_body, FixtureSourceError};
use reqwest::{Client, Url};
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	operation::Operation,
	source::Source,
};

const FIXTURE_ORIGIN: &str = "http://127.0.0.1:18787";
const LISTEN_ADDRESS: &str = "127.0.0.1:18788";

#[derive(Clone)]
struct RouteState {
	origin: Url,
	operation: Arc<Operation<Url, u32>>,
}

#[derive(Debug, thiserror::Error)]
enum NativeError {
	#[error("could not configure the fixture HTTP client")]
	Client(#[source] reqwest::Error),
	#[error("invalid loopback fixture origin")]
	Origin,
	#[error("the loopback listener could not start or stopped with an error")]
	Listener(#[source] std::io::Error),
}

/// Builds the shared operation with the native source and listens on loopback.
#[tokio::main]
async fn main() -> Result<(), NativeError> {
	todo!("build the fixture client and run the Axum router")
}

fn router(state: RouteState) -> Router {
	Router::new()
		.route("/experiment/{fixture}", get(serve_fixture))
		.with_state(state)
}

/// Parses the fixture path, runs the shared operation, and returns the same
/// completed JSON reply as the Worker's Durable Object. An unknown fixture is 404.
async fn serve_fixture(Path(_segment): Path<String>, State(_state): State<RouteState>) -> Response {
	todo!("serve the shared fixture route")
}

struct NativeSource {
	client: Client,
}

impl NativeSource {
	/// Configures reqwest with `Policy::none()` so it returns the original 302.
	fn new() -> Result<Self, reqwest::Error> {
		todo!("build a redirect-disabled native HTTP client")
	}
}

impl Source<Url> for NativeSource {
	type Response = reqwest::Response;
	type Error = FixtureSourceError<reqwest::Error>;

	fn retrieve<'a>(&'a self, _url: &'a Url) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(async { todo!("fetch and check the original HTTP status") })
	}
}

struct NativeAdapter;

impl AdaptResponse<reqwest::Response> for NativeAdapter {
	type Metadata = ();
	type Error = reqwest::Error;

	fn adapt_response(
		&self,
		response: reqwest::Response,
	) -> BoxFuture<'_, Result<BodyWithMetadata<()>, Self::Error>> {
		Box::pin(async move { response.bytes().await.map(buffered_fixture_body) })
	}
}
