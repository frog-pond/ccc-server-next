//! Workers invokes these entry points. The Durable Object stores the fixture job
//! and completed reply in SQLite and invokes the shared Trifecta operation from
//! its alarm.

use std::{cell::Cell, error::Error, future::Future, rc::Rc};

use bytes::Bytes;
use fixture_experiment_route::{
	buffered_fixture_body, fixture_operation, Fixture, FixtureSourceError, NonSuccessStatus,
	ObjectReply, RefreshFailure, RefreshFailureEvent, RefreshJob, RefreshOutcome,
};
use futures_util::FutureExt;
use serde::Serialize;
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};
use worker::{
	durable_object, event, send::SendFuture, wasm_bindgen, Context, DurableObject, Env, Fetch,
	Headers, Method, Request, RequestInit, RequestRedirect, Response, SqlStorage, SqlStorageValue,
	State, Storage,
};

use address::LoopbackUrl;

const FIXTURES: &str = "FIXTURES";
const ALARM_DELAY_MS: i64 = 3_000;

type SourceFailure = FixtureSourceError<worker::Error>;

/// Forwards a known GET to its fixture object. Unknown fixtures return the
/// native route's 404 and other methods return its 405 with `Allow: GET`, without
/// contacting an object.
#[event(fetch)]
pub async fn fetch(request: Request, env: Env, _context: Context) -> worker::Result<Response> {
	let Ok(fixture) = Fixture::from_route_path(&request.path()) else {
		return Response::empty().map(|response| response.with_status(404));
	};
	if request.method() != Method::Get {
		let headers = Headers::new();
		headers.set("Allow", "GET")?;
		return Response::empty().map(|response| response.with_status(405).with_headers(headers));
	}

	let object = env
		.durable_object(FIXTURES)?
		.get_by_name(fixture.object_name())?;
	object.fetch_with_request(request).await
}

/// Owns the SQLite storage and alarm for one fixture.
#[durable_object(alarm)]
pub struct FixtureObject {
	state: State,
	env: Env,
	// get_alarm() can return None while the alarm handler is running.
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

	async fn fetch(&self, request: Request) -> worker::Result<Response> {
		let Some(owner) = self.owner() else {
			return reply(ObjectReply::StoredRecordInvalid);
		};
		let Ok(requested) = Fixture::from_route_path(&request.path()) else {
			return Response::empty().map(|response| response.with_status(404));
		};
		let storage = self.state.storage();
		let alarm_running = Rc::clone(&self.alarm_running);
		// Keep the storage/alarm lookup and re-arm together. Return routine
		// failures as values so they do not fail the concurrency gate.
		let result = self
			.state
			.block_concurrency_while(async move {
				Ok(read_or_schedule(&storage, owner, requested, &alarm_running).await)
			})
			.await;
		match result {
			Ok(Ok(ReadReply::Complete(payload))) => completed_response(payload),
			Ok(Ok(ReadReply::Pending)) => reply(ObjectReply::Pending),
			Ok(Err(failure)) => log_and_reply(owner, &failure),
			Err(error) => log_and_reply(owner, &StorageOperation::Gate.failure(error)),
		}
	}

	/// Runs the stored parameters. The object name identifies the fixture in
	/// logs and guards against a job from another object. A mismatch is
	/// logged as an invalid record and a later fetch returns that failure.
	async fn alarm(&self) -> worker::Result<Response> {
		let _running = RunningAlarm::start(&self.alarm_running);
		let Some(owner) = self.owner() else {
			return Response::empty();
		};
		match self.refresh(owner).await {
			Ok(()) => Response::empty(),
			Err(failure) => {
				record_object_failure(owner, &failure);
				match failure {
					ObjectFailure::Storage { cause, .. } => Err(cause),
					_ => Response::empty(),
				}
			}
		}
	}
}

impl FixtureObject {
	fn owner(&self) -> Option<Fixture> {
		let name = self.state.id().name();
		let owner = name
			.as_deref()
			.and_then(|name| Fixture::from_object_name(name).ok());
		if owner.is_none() {
			log_object_failure(
				name.as_deref().unwrap_or("unknown"),
				"identify_object",
				"invalid_object",
				"object has no recognized fixture name",
			);
		}
		owner
	}

	async fn refresh(&self, owner: Fixture) -> Result<(), ObjectFailure> {
		let records = Records::open(self.state.storage().sql())?;
		let Some(requested) = records.read_job(owner)? else {
			return Ok(());
		};
		if records.read(Record::Outcome)?.is_some() {
			return Ok(());
		}
		let result = run_refresh(&self.env, requested).await;
		records.write(Record::Outcome, &ObjectReply::from(result))
	}
}

struct RunningAlarm<'a>(&'a Cell<bool>);

impl<'a> RunningAlarm<'a> {
	fn start(running: &'a Cell<bool>) -> Self {
		running.set(true);
		Self(running)
	}
}

impl Drop for RunningAlarm<'_> {
	fn drop(&mut self) {
		self.0.set(false);
	}
}

enum ReadReply {
	Pending,
	Complete(String),
}

async fn read_or_schedule(
	storage: &Storage,
	owner: Fixture,
	requested: Fixture,
	alarm_running: &Cell<bool>,
) -> Result<ReadReply, ObjectFailure> {
	let records = Records::open(storage.sql())?;
	let job = records.read_job(owner)?;
	if let Some(payload) = records.read(Record::Outcome)? {
		return Ok(ReadReply::Complete(payload));
	}
	if job.is_none() {
		check_owner(owner, requested)?;
		records.write(Record::Job, &RefreshJob::ReadFixture(requested))?;
	}

	let scheduled = storage
		.get_alarm()
		.await
		.map_err(|cause| StorageOperation::ReadAlarm.failure(cause))?;
	if scheduled.is_none() && !alarm_running.get() {
		storage
			.set_alarm(ALARM_DELAY_MS)
			.await
			.map_err(ObjectFailure::ScheduleAlarm)?;
	}
	Ok(ReadReply::Pending)
}

#[derive(Clone, Copy)]
enum Record {
	Job,
	Outcome,
}

impl Record {
	const fn key(self) -> &'static str {
		match self {
			Self::Job => "job",
			Self::Outcome => "outcome",
		}
	}
}

struct Records(SqlStorage);

impl Records {
	fn open(sql: SqlStorage) -> Result<Self, ObjectFailure> {
		sql
			.exec(
				"CREATE TABLE IF NOT EXISTS fixture_records (
				name TEXT PRIMARY KEY,
				payload TEXT NOT NULL CHECK (json_valid(payload))
			) STRICT",
				None,
			)
			.map_err(|cause| StorageOperation::Open.failure(cause))?;
		Ok(Self(sql))
	}

	fn read(&self, record: Record) -> Result<Option<String>, ObjectFailure> {
		let on_error = |cause| StorageOperation::Read(record).failure(cause);
		let cursor = self
			.0
			.exec(
				"SELECT payload FROM fixture_records WHERE name = ?",
				vec![record.key().into()],
			)
			.map_err(on_error)?;
		match cursor.raw().next().transpose().map_err(on_error)? {
			Some(row) => match row.into_iter().next() {
				Some(SqlStorageValue::String(payload)) => Ok(Some(payload)),
				_ => Err(ObjectFailure::RecordType(record)),
			},
			None => Ok(None),
		}
	}

	fn read_job(&self, owner: Fixture) -> Result<Option<Fixture>, ObjectFailure> {
		let Some(payload) = self.read(Record::Job)? else {
			return Ok(None);
		};
		let RefreshJob::ReadFixture(requested) =
			serde_json::from_str(&payload).map_err(ObjectFailure::DecodeJob)?;
		check_owner(owner, requested)?;
		Ok(Some(requested))
	}

	fn write(&self, record: Record, value: &impl Serialize) -> Result<(), ObjectFailure> {
		let payload =
			serde_json::to_string(value).map_err(|cause| ObjectFailure::Encode { record, cause })?;
		self
			.0
			.exec(
				"INSERT INTO fixture_records (name, payload) VALUES (?, ?)
			 ON CONFLICT(name) DO UPDATE SET payload = excluded.payload",
				vec![record.key().into(), payload.into()],
			)
			.map_err(|cause| StorageOperation::Write(record).failure(cause))?;
		Ok(())
	}
}

fn check_owner(owner: Fixture, requested: Fixture) -> Result<(), ObjectFailure> {
	if owner == requested {
		Ok(())
	} else {
		Err(ObjectFailure::JobFixtureMismatch(requested))
	}
}

/// Uses the already encoded SQLite text directly as the HTTP JSON body.
fn completed_response(payload: String) -> worker::Result<Response> {
	let headers = Headers::new();
	headers.set("Content-Type", "application/json")?;
	Response::from_bytes(payload.into_bytes()).map(|response| response.with_headers(headers))
}

fn reply(reply: ObjectReply) -> worker::Result<Response> {
	let status = match reply {
		ObjectReply::Complete { .. } => 200,
		ObjectReply::Pending => 202,
		ObjectReply::StorageUnavailable | ObjectReply::AlarmSchedulingFailed => 503,
		ObjectReply::StoredRecordInvalid => 500,
	};
	Response::from_json(&reply).map(|response| response.with_status(status))
}

fn log_and_reply(fixture: Fixture, failure: &ObjectFailure) -> worker::Result<Response> {
	record_object_failure(fixture, failure);
	reply(failure.reply())
}

mod address {
	use fixture_experiment_route::Fixture;
	use std::net::IpAddr;
	use worker::Url;

	use super::FixtureSetupFailure;
	/// The checked URL is only constructed from a numeric HTTP loopback origin.
	pub(super) struct LoopbackUrl(Url);

	impl LoopbackUrl {
		pub(super) fn for_fixture(origin: &str, fixture: Fixture) -> Result<Self, FixtureSetupFailure> {
			let mut url = Url::parse(origin).map_err(|_| FixtureSetupFailure::InvalidLoopbackUrl)?;
			let host = url.host_str().unwrap_or_default();
			let loopback = host
				.trim_start_matches('[')
				.trim_end_matches(']')
				.parse::<IpAddr>()
				.is_ok_and(|ip| ip.is_loopback());
			if url.scheme() != "http"
				|| !loopback
				|| !url.username().is_empty()
				|| url.password().is_some()
				|| url.path() != "/"
				|| url.query().is_some()
				|| url.fragment().is_some()
			{
				return Err(FixtureSetupFailure::InvalidLoopbackUrl);
			}
			url.set_path(fixture.segment());
			Ok(Self(url))
		}

		pub(super) fn as_str(&self) -> &str {
			self.0.as_str()
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum FixtureSetupFailure {
	#[error("EXPERIMENT_FIXTURE_ORIGIN binding is missing or unreadable")]
	Binding(#[source] worker::Error),
	#[error("fixture origin must be a valid loopback HTTP URL")]
	InvalidLoopbackUrl,
}

fn prepare_fixture(env: &Env, fixture: Fixture) -> Result<LoopbackUrl, FixtureSetupFailure> {
	let origin = env
		.var("EXPERIMENT_FIXTURE_ORIGIN")
		.map_err(FixtureSetupFailure::Binding)?;
	LoopbackUrl::for_fixture(&origin.to_string(), fixture)
}

async fn run_refresh(env: &Env, fixture: Fixture) -> RefreshOutcome {
	let url = match prepare_fixture(env, fixture) {
		Ok(url) => url,
		Err(error) => return failed_refresh(fixture, RefreshFailure::FixtureSetup, &error),
	};
	match fixture_operation(FixtureSource, FixtureAdapter)
		.perform(url)
		.await
	{
		Ok(value) => RefreshOutcome::Value { value },
		Err(error) => failed_refresh(fixture, error.failure, &error.cause),
	}
}

fn failed_refresh(
	fixture: Fixture,
	failure: RefreshFailure,
	error: &(dyn Error + 'static),
) -> RefreshOutcome {
	let cause = error.source().unwrap_or(error);
	let diagnostic = if let Some(error) = cause.downcast_ref::<worker::Error>().or_else(|| {
		cause
			.source()
			.and_then(|source| source.downcast_ref::<worker::Error>())
	}) {
		let kind: &'static str = error.into();
		kind.to_owned()
	} else {
		match cause.source() {
			Some(source) => format!("{cause}: {source}"),
			None => cause.to_string(),
		}
	};
	log_event(&RefreshFailureEvent::new(fixture, failure, &diagnostic));
	RefreshOutcome::Failed { failure }
}

struct FixtureSource;

impl Source<LoopbackUrl> for FixtureSource {
	type Response = Response;
	type Error = SourceFailure;

	fn retrieve<'a>(
		&'a self,
		url: &'a LoopbackUrl,
	) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(fetch_fixture(url))
	}
}

/// The SDK wrapper supplies the `Send` required by Trifecta on this
/// single-threaded runtime. The native response already satisfies `Send`.
fn fetch_fixture(
	url: &LoopbackUrl,
) -> SendFuture<impl Future<Output = Result<Response, SourceFailure>> + '_> {
	SendFuture::new(async move {
		let mut init = RequestInit::new();
		init.with_redirect(RequestRedirect::Manual);
		let request = Request::new_with_init(url.as_str(), &init).map_err(SourceFailure::Fetch)?;
		let response = Fetch::Request(request)
			.send()
			.await
			.map_err(SourceFailure::Fetch)?;
		let status = response.status_code();
		if (200..300).contains(&status) {
			return Ok(response);
		}
		let status = NonSuccessStatus::try_from(status).map_err(SourceFailure::InvalidStatus)?;
		Err(SourceFailure::HttpStatus(status))
	})
}

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

/// Wraps the SDK body-read future and gives ownership of the resulting
/// `Vec<u8>` to `Bytes`. The shared one-chunk stream then owns that allocation.
fn read_fixture_body(
	mut response: Response,
) -> SendFuture<impl Future<Output = worker::Result<Bytes>>> {
	SendFuture::new(async move { response.bytes().await.map(Bytes::from) })
}

#[derive(Clone, Copy)]
enum StorageOperation {
	Open,
	Read(Record),
	Write(Record),
	ReadAlarm,
	Gate,
}

impl StorageOperation {
	const fn name(self) -> &'static str {
		match self {
			Self::Open => "open_storage",
			Self::Read(Record::Job) => "read_job",
			Self::Read(Record::Outcome) => "read_outcome",
			Self::Write(Record::Job) => "write_job",
			Self::Write(Record::Outcome) => "write_outcome",
			Self::ReadAlarm => "read_alarm",
			Self::Gate => "concurrency_gate",
		}
	}

	const fn failure(self, cause: worker::Error) -> ObjectFailure {
		ObjectFailure::Storage {
			operation: self,
			cause,
		}
	}
}

enum ObjectFailure {
	Storage {
		operation: StorageOperation,
		cause: worker::Error,
	},
	ScheduleAlarm(worker::Error),
	DecodeJob(serde_json::Error),
	RecordType(Record),
	JobFixtureMismatch(Fixture),
	Encode {
		record: Record,
		cause: serde_json::Error,
	},
}

impl ObjectFailure {
	const fn reply(&self) -> ObjectReply {
		match self {
			Self::Storage { .. } | Self::Encode { .. } => ObjectReply::StorageUnavailable,
			Self::ScheduleAlarm(_) => ObjectReply::AlarmSchedulingFailed,
			Self::DecodeJob(_) | Self::RecordType(_) | Self::JobFixtureMismatch(_) => {
				ObjectReply::StoredRecordInvalid
			}
		}
	}
}

#[derive(Serialize)]
struct ObjectFailureEvent<'a> {
	event: &'static str,
	fixture: &'a str,
	operation: &'static str,
	kind: &'static str,
	cause: &'a str,
}

/// Records the fixture and operation before reporting a failed reply or
/// propagating a storage error to the alarm runtime for retry.
fn record_object_failure(fixture: Fixture, failure: &ObjectFailure) {
	let fixture = fixture.segment();
	match failure {
		ObjectFailure::Storage { operation, cause } => {
			log_object_failure(fixture, operation.name(), "storage", cause.into());
		}
		ObjectFailure::ScheduleAlarm(cause) => {
			log_object_failure(fixture, "schedule_alarm", "schedule_alarm", cause.into());
		}
		ObjectFailure::DecodeJob(cause) => {
			log_object_failure(fixture, "decode_job", "decode_record", &cause.to_string());
		}
		ObjectFailure::RecordType(record) => log_object_failure(
			fixture,
			StorageOperation::Read(*record).name(),
			"decode_record",
			"stored value is not text",
		),
		ObjectFailure::JobFixtureMismatch(job) => {
			log_object_failure(fixture, "validate_job", "decode_record", job.segment());
		}
		ObjectFailure::Encode { record, cause } => log_object_failure(
			fixture,
			StorageOperation::Write(*record).name(),
			"encode_record",
			&cause.to_string(),
		),
	}
}

fn log_object_failure(fixture: &str, operation: &'static str, kind: &'static str, cause: &str) {
	log_event(&ObjectFailureEvent {
		event: "worker_experiment.object_failed",
		fixture,
		operation,
		kind,
		cause,
	});
}

fn log_event(event: &impl Serialize) {
	match serde_json::to_string(event) {
		Ok(json) => worker::console_error!("{json}"),
		Err(cause) => worker::console_error!("could not encode fixture failure event: {cause}"),
	}
}
