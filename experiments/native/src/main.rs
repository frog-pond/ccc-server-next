//! Tokio listens on loopback and Axum runs the shared fixture operation.

use std::{error::Error, sync::Arc, time::Duration};

use axum::{
	extract::State,
	http::{header, Method, StatusCode, Uri},
	response::{IntoResponse, Response},
	routing::any,
	Json, Router,
};
use fixture_experiment_route::{
	buffered_fixture_body, fixture_operation, Fixture, FixtureOperation, FixtureSourceError,
	NonSuccessStatus, ObjectReply, RefreshFailureEvent, RefreshOutcome,
};
use reqwest::{Client, Url};
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};

const FIXTURE_ORIGIN: &str = "http://127.0.0.1:18787";
const LISTEN_ADDRESS: &str = "127.0.0.1:18788";

#[derive(Clone)]
struct RouteState {
	origin: Url,
	operation: Arc<FixtureOperation<Url, reqwest::Error>>,
}

#[derive(Debug, thiserror::Error)]
enum NativeError {
	#[error("could not configure the fixture HTTP client")]
	Client(#[source] reqwest::Error),
	#[error("invalid loopback fixture origin")]
	Origin,
	#[error("the loopback listener could not start or stopped with an error")]
	Listener(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), NativeError> {
	let source = NativeSource::new().map_err(NativeError::Client)?;
	let state = RouteState {
		origin: Url::parse(FIXTURE_ORIGIN).map_err(|_| NativeError::Origin)?,
		operation: Arc::new(fixture_operation(source, NativeAdapter)),
	};
	let listener = tokio::net::TcpListener::bind(LISTEN_ADDRESS).await?;
	let app = Router::new()
		.route("/experiment/{fixture}", any(serve_fixture))
		.with_state(state);
	axum::serve(listener, app).await?;
	Ok(())
}

/// An unknown fixture returns 404 and a non-GET to a known fixture returns 405.
async fn serve_fixture(State(state): State<RouteState>, method: Method, uri: Uri) -> Response {
	let Ok(fixture) = Fixture::from_route_path(uri.path()) else {
		return StatusCode::NOT_FOUND.into_response();
	};
	if method != Method::GET {
		return (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "GET")]).into_response();
	}

	let mut url = state.origin.clone();
	url.set_path(fixture.segment());
	let result = match state.operation.perform(url).await {
		Ok(value) => RefreshOutcome::Value { value },
		Err(error) => {
			let diagnostic = match error.cause.source().and_then(Error::source) {
				Some(source) => format!("{}: {source}", error.cause),
				None => error.cause.to_string(),
			};
			let event = RefreshFailureEvent::new(fixture, error.failure, &diagnostic);
			match serde_json::to_string(&event) {
				Ok(json) => eprintln!("{json}"),
				Err(cause) => eprintln!("could not encode fixture failure event: {cause}"),
			}
			RefreshOutcome::Failed {
				failure: error.failure,
			}
		}
	};
	Json(ObjectReply::from(result)).into_response()
}

struct NativeSource {
	client: Client,
}

impl NativeSource {
	/// Disables redirects and system proxies so requests stay on loopback.
	fn new() -> Result<Self, reqwest::Error> {
		Client::builder()
			.redirect(reqwest::redirect::Policy::none())
			.no_proxy()
			.timeout(Duration::from_secs(5))
			.build()
			.map(|client| Self { client })
	}
}

impl Source<Url> for NativeSource {
	type Response = reqwest::Response;
	type Error = FixtureSourceError<reqwest::Error>;

	fn retrieve<'a>(&'a self, url: &'a Url) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(async move {
			let response = self
				.client
				.get(url.clone())
				.send()
				.await
				.map_err(|error| FixtureSourceError::Fetch(error.without_url()))?;
			if response.status().is_success() {
				return Ok(response);
			}
			let status = NonSuccessStatus::try_from(response.status().as_u16())
				.map_err(FixtureSourceError::InvalidStatus)?;
			Err(FixtureSourceError::HttpStatus(status))
		})
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
		Box::pin(async move {
			response
				.bytes()
				.await
				.map(buffered_fixture_body)
				.map_err(reqwest::Error::without_url)
		})
	}
}
