//! Worker entry points and adapters for the pinned Trifecta draft.

// These signatures are under review before the Worker bodies are written.
#![allow(dead_code)]

use std::future::Future;

use trifecta::{
	body::{AdaptResponse, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	operation::Error as OperationError,
	source::Source,
	transform::Transform,
};
use worker::{
	durable_object, event, send::SendFuture, wasm_bindgen, Context, DurableObject, Env, Request,
	Response, State, Url,
};

use crate::{Fixture, InvalidNonSuccessStatus, NonSuccessStatus, RefreshFailure};

/// Serves `GET /experiment/{value|redirect}` from the fixture's Durable Object.
///
/// # Errors
///
/// Returns a Worker error when the runtime cannot resolve or call the object.
/// Expected object failures cross the call as a serialized [`crate::ObjectReply`].
#[event(fetch)]
pub async fn fetch(_request: Request, _env: Env, _context: Context) -> worker::Result<Response> {
	todo!("dispatch the experiment route to the fixture's Durable Object")
}

/// Owns the SQLite storage and alarm of a single loopback fixture.
#[durable_object(alarm)]
pub struct FixtureObject {
	state: State,
	env: Env,
}

impl DurableObject for FixtureObject {
	fn new(state: State, env: Env) -> Self {
		Self { state, env }
	}

	/// Reads the fixture's recorded result or stores the initial job and schedules
	/// its alarm. A SQLite read/write failure, a rejected alarm schedule and an
	/// undecodable JSON record each have their own [`crate::ObjectReply`] case.
	///
	/// # Errors
	///
	/// Returns a Worker error if the reply cannot be encoded as a response.
	async fn fetch(&self, _request: Request) -> worker::Result<Response> {
		todo!("read or schedule and return the outcome as HTTP JSON")
	}

	/// Rebuilds the refresh from the stored job and the current fixture binding.
	/// An upstream or transform failure is logged and stored as a result.
	///
	/// # Errors
	///
	/// Returns a Worker error on storage access failures so the runtime can
	/// retry the alarm. No completed result exists until storage accepts it.
	async fn alarm(&self) -> worker::Result<Response> {
		todo!("load the durable job, run Trifecta and store the outcome")
	}
}

/// Fetches the prepared loopback URL with `RequestRedirect::Manual`.
struct FixtureSource;

#[derive(Debug, thiserror::Error)]
enum SourceFailure {
	#[error("Workers Fetch failed")]
	Fetch(#[source] worker::Error),
	#[error("upstream returned HTTP {0}")]
	HttpStatus(NonSuccessStatus),
	#[error("Workers Fetch returned an invalid HTTP status")]
	InvalidStatus(#[source] InvalidNonSuccessStatus),
}

impl Source<Url> for FixtureSource {
	type Response = Response;
	type Error = SourceFailure;

	fn retrieve<'a>(&'a self, url: &'a Url) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(fetch_fixture(url))
	}
}

/// The SDK's `SendFuture` supplies the `Send` required by Trifecta's `BoxFuture`
/// on the single-threaded Worker. The native `Response` already satisfies `Send`.
fn fetch_fixture(
	_url: &Url,
) -> SendFuture<impl Future<Output = Result<Response, SourceFailure>> + '_> {
	SendFuture::new(async { todo!("fetch with manual redirects and check the response status") })
}

/// Reads the small fixture body into Trifecta's byte stream.
struct FixtureAdapter;

impl AdaptResponse<Response> for FixtureAdapter {
	type Metadata = ();
	type Error = worker::Error;

	fn adapt_response(
		&self,
		response: Response,
	) -> BoxFuture<'_, Result<BodyWithMetadata<()>, Self::Error>> {
		Box::pin(read_fixture_body(response))
	}
}

/// The SDK's body-read future also needs `SendFuture` before Trifecta can box it.
fn read_fixture_body(
	_response: Response,
) -> SendFuture<impl Future<Output = worker::Result<BodyWithMetadata<()>>>> {
	SendFuture::new(async { todo!("read the Workers response body once") })
}

/// Decodes the fixture's JSON integer after the adapter removes Worker types.
struct FixtureTransform;

#[derive(Debug, thiserror::Error)]
enum TransformFailure {
	#[error("failed to consume the adapted body")]
	ReadBody(#[source] BodyReadError),
	#[error("fixture body was not a JSON unsigned integer")]
	Decode(#[source] serde_json::Error),
}

impl Transform<Url, (), u32> for FixtureTransform {
	type Error = TransformFailure;

	fn transform<'a>(
		&'a self,
		_url: &'a Url,
		_body: BodyWithMetadata<()>,
	) -> BoxFuture<'a, Result<u32, Self::Error>> {
		Box::pin(async { todo!("decode the integer from the adapted body") })
	}
}

/// Classifies an uncached operation error for SQLite storage and the HTTP reply.
///
/// `Source` downcasts the error source to `SourceFailure`. `HttpStatus` keeps
/// the checked status read from `worker::Response::status_code()`. Transport
/// errors and invalid HTTP statuses become `Fetch`. `ResponseAdaptation`
/// becomes `ReadBody`. `Transformation` downcasts to `TransformFailure` and
/// becomes `ReadBody` or `Decode`. An unexpected source type, cache or flight
/// stage, or future `ErrorKind` becomes `Pipeline`.
fn classify_refresh_failure(_error: &OperationError) -> RefreshFailure {
	todo!("classify the stage and downcast its cause where needed")
}

/// Emits `worker_experiment.refresh_failed` before the alarm stores a failure.
/// The structured event contains `fixture`, `failure` (the wire classification)
/// and `cause` (the underlying diagnostic). Cause formatting excludes request
/// URLs, headers and response bodies. The stored result carries the classification.
fn record_refresh_failure(_fixture: Fixture, _failure: RefreshFailure, _cause: &OperationError) {
	todo!("emit the structured refresh failure event")
}
