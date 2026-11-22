//! The Workers runtime invokes the route and Durable Object entry points here.
//! The object handlers use its SQLite storage and alarm API, and the alarm
//! invokes the shared Trifecta operation through the local Fetch adapter.

// These signatures are under review before the Worker bodies are written.
#![allow(dead_code)]

use std::{cell::Cell, future::Future, rc::Rc};

use bytes::Bytes;
use fixture_experiment_route::{
	buffered_fixture_body, Fixture, FixtureSourceError, RefreshFailure,
};
use futures_util::FutureExt;
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	operation::Error as OperationError,
	source::Source,
};
use worker::{
	durable_object, event, send::SendFuture, wasm_bindgen, Context, DurableObject, Env, Request,
	Response, State, Url,
};

type SourceFailure = FixtureSourceError<worker::Error>;

/// Serves `GET /experiment/{value|redirect}` from the fixture's Durable Object.
///
/// # Errors
///
/// Returns a Worker error when the runtime cannot resolve or call the object.
/// Expected object failures cross the call as a serialized
/// [`fixture_experiment_route::ObjectReply`].
#[event(fetch)]
pub async fn fetch(_request: Request, _env: Env, _context: Context) -> worker::Result<Response> {
	todo!("dispatch the experiment route to the fixture's Durable Object")
}

/// Owns the SQLite storage and alarm of a single loopback fixture.
#[durable_object(alarm)]
pub struct FixtureObject {
	state: State,
	env: Env,
	// get_alarm can return None while the alarm handler is running.
	alarm_running: Rc<Cell<bool>>,
}

impl DurableObject for FixtureObject {
	fn new(state: State, env: Env) -> Self {
		Self {
			state,
			env,
			alarm_running: Rc::new(Cell::new(false)),
		}
	}

	/// Reads the fixture's recorded result or stores the initial job and schedules
	/// its alarm. For an unfinished job, serializes the check and re-arm with
	/// respect to other incoming events. Reuses a scheduled or running alarm and
	/// otherwise schedules the stored job again. A storage, scheduling or record
	/// decoding failure is logged and returned as a distinct reply.
	///
	/// # Errors
	///
	/// Returns a Worker error if the reply cannot be encoded as a response.
	async fn fetch(&self, _request: Request) -> worker::Result<Response> {
		todo!("read or schedule and return the outcome as HTTP JSON")
	}

	/// Marks the alarm running until the handler exits, including on failure.
	/// Reads the stored job and fixture binding, then runs the shared operation.
	/// Duplicate deliveries skip work when the outcome is already stored. Setup,
	/// upstream and transform failures are logged and stored as completed results.
	///
	/// # Errors
	///
	/// Logs and returns storage errors so the runtime can retry the alarm. Logs
	/// an undecodable record and leaves it for a later `fetch` to report.
	async fn alarm(&self) -> worker::Result<Response> {
		todo!("load the durable job, run Trifecta and store the outcome")
	}
}

/// Fetches the prepared loopback URL with `RequestRedirect::Manual`.
struct FixtureSource;

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
		Box::pin(read_fixture_body(response).map(|result| result.map(buffered_fixture_body)))
	}
}

/// The SDK's body-read future also needs `SendFuture` before Trifecta can box it.
/// Transfers the SDK's `Vec<u8>` into `Bytes` by ownership. The adapter moves
/// those bytes into the shared one-chunk stream without copying the payload.
fn read_fixture_body(
	_response: Response,
) -> SendFuture<impl Future<Output = worker::Result<Bytes>>> {
	SendFuture::new(async { todo!("read the Workers response body once") })
}

#[derive(Debug, thiserror::Error)]
enum FixtureSetupFailure {
	#[error("EXPERIMENT_FIXTURE_ORIGIN binding is missing or unreadable")]
	Binding(#[source] worker::Error),
	#[error("fixture origin must be a valid loopback HTTP URL")]
	InvalidLoopbackUrl,
}

/// Reads the fixture identity from the object's name, including during alarm
/// delivery when the stored job itself may be unreadable.
fn object_fixture(state: &State) -> Option<Fixture> {
	state
		.id()
		.name()
		.as_deref()
		.and_then(Fixture::from_object_name)
}

/// Reads the local binding and prepares the URL before starting Trifecta.
///
/// # Errors
///
/// A missing binding or an invalid or non-loopback URL becomes a stored
/// completed failure of kind `FixtureSetup`, with its cause logged first.
fn prepare_fixture(_env: &Env, _fixture: Fixture) -> Result<Url, FixtureSetupFailure> {
	todo!("validate the configured origin and append the fixture path")
}

enum RefreshDiagnostic<'a> {
	Setup(&'a FixtureSetupFailure),
	Operation(&'a OperationError),
}

/// Emits `worker_experiment.refresh_failed` before the alarm stores a failure.
/// The structured event contains `fixture`, `failure` (the wire classification)
/// and `cause` (the underlying diagnostic). Cause formatting excludes request
/// URLs, headers and response bodies. The stored result carries the classification.
fn record_refresh_failure(
	_fixture: Fixture,
	_failure: RefreshFailure,
	_cause: RefreshDiagnostic<'_>,
) {
	todo!("emit the structured refresh failure event")
}

#[derive(Clone, Copy)]
enum StorageOperation {
	ReadJob,
	WriteJob,
	ReadOutcome,
	WriteOutcome,
	ReadAlarm,
}

#[derive(Clone, Copy)]
enum StoredRecord {
	Job,
	Outcome,
}

enum ObjectFailure<'a> {
	Storage {
		operation: StorageOperation,
		cause: &'a worker::Error,
	},
	ScheduleAlarm(&'a worker::Error),
	Decode {
		record: StoredRecord,
		cause: &'a serde_json::Error,
	},
}

/// Emits `worker_experiment.object_failed` before returning an object failure,
/// including errors returned to the runtime to retry an alarm. The event has
/// `fixture`, `operation`, `kind`, and a diagnostic `cause`. Operations are
/// `read_job`, `write_job`, `read_outcome`, `write_outcome`, `read_alarm`,
/// `schedule_alarm`, `decode_job`, and `decode_outcome`. Kinds are `storage`,
/// `schedule_alarm`, and `decode_record`. Diagnostics omit stored payloads.
fn record_object_failure(_fixture: Fixture, _failure: ObjectFailure<'_>) {
	todo!("emit the structured object failure event")
}
