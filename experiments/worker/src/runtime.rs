//! Workers invokes these entry points. The Durable Object stores the fixture job
//! and completed reply in SQLite and invokes the shared Trifecta operation from
//! its alarm.

use std::{cell::Cell, error::Error, future::Future, pin::pin, rc::Rc};

use bytes::Bytes;
use fixture_experiment_route::{
	buffered_fixture_body, fixture_operation, Fixture, FixtureSourceError, NonSuccessStatus,
	ObjectReply, RefreshFailure, RefreshFailureEvent, RefreshJob, RefreshOutcome, UnknownFixture,
	REFRESH_DEADLINE,
};
use futures_util::{
	future::{select, Either},
	FutureExt,
};
use serde::Serialize;
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};
use worker::{
	durable_object, event, send::SendFuture, wasm_bindgen, web_sys::AbortController, AbortSignal,
	Context, Date, Delay, DurableObject, Env, Fetch, Headers, Method, Request, RequestInit,
	RequestRedirect, Response, SqlStorage, SqlStorageValue, State, Storage,
};

use address::LoopbackUrl;

const FIXTURES: &str = "FIXTURES";
const ALARM_DELAY_MS: i64 = 3_000;
const OUTCOME_LIFETIME_MS: i64 = 5_000;

type SourceFailure = FixtureSourceError<worker::Error>;

/// Forwards a known GET to its fixture object. Unknown fixtures return the
/// native route's 404 and other methods return its 405 with `Allow: GET`, without
/// contacting an object.
///
/// # Errors
///
/// Returns a Workers SDK error if a local response or its headers cannot be
/// constructed. Object call failures are logged and become a JSON 503.
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

	match call_fixture_object(request, &env, fixture).await {
		Ok(response) => Ok(response),
		Err(failure) => {
			log_event(&ObjectFailureEvent {
				event: "worker_experiment.object_call_failed",
				fixture: Some(fixture),
				object_name: None,
				operation: failure.operation.name(),
				kind: "object_call",
				cause: (&failure.cause).into(),
			});
			reply(ObjectReply::ObjectCallFailed)
		}
	}
}

async fn call_fixture_object(
	request: Request,
	env: &Env,
	fixture: Fixture,
) -> Result<Response, ObjectCallFailure> {
	let namespace = env
		.durable_object(FIXTURES)
		.map_err(|cause| ObjectCallOperation::ResolveBinding.failure(cause))?;
	let object = namespace
		.get_by_name(fixture.object_name())
		.map_err(|cause| ObjectCallOperation::GetByName.failure(cause))?;
	object
		.fetch_with_request(request)
		.await
		.map_err(|cause| ObjectCallOperation::FetchWithRequest.failure(cause))
}

#[derive(Clone, Copy, Debug)]
enum ObjectCallOperation {
	ResolveBinding,
	GetByName,
	FetchWithRequest,
}

impl ObjectCallOperation {
	const fn name(self) -> &'static str {
		match self {
			Self::ResolveBinding => "resolve_binding",
			Self::GetByName => "get_by_name",
			Self::FetchWithRequest => "fetch_with_request",
		}
	}

	const fn failure(self, cause: worker::Error) -> ObjectCallFailure {
		ObjectCallFailure {
			operation: self,
			cause,
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("fixture object call failed during {operation:?}")]
struct ObjectCallFailure {
	operation: ObjectCallOperation,
	#[source]
	cause: worker::Error,
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
		let Some(owner) = self.fetch_owner() else {
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

	/// Runs the stored parameters. A nameless alarm trusts the stored job.
	/// When present, the object name identifies the fixture and guards against
	/// a job from another object. A mismatch is logged as an invalid record.
	async fn alarm(&self) -> worker::Result<Response> {
		let _running = RunningAlarm::start(&self.alarm_running);
		let Ok(owner) = self.owner() else {
			return Response::empty();
		};
		let storage = self.state.storage();
		let loaded = Records::open(storage.sql()).and_then(|records| {
			let job = records.read_job(owner)?;
			Ok((records, job))
		});
		let (records, requested) = match loaded {
			Ok((records, Some(requested))) => (records, requested),
			Ok((_, None)) => return Response::empty(),
			Err(failure) => return alarm_failure(owner, failure),
		};
		match self.refresh(&storage, &records, requested).await {
			Ok(()) => Response::empty(),
			Err(failure) => alarm_failure(Some(requested), failure),
		}
	}
}

fn alarm_failure(fixture: Option<Fixture>, failure: ObjectFailure) -> worker::Result<Response> {
	record_object_failure(fixture, &failure);
	match failure {
		ObjectFailure::Storage { cause, .. } | ObjectFailure::ScheduleAlarm(cause) => Err(cause),
		_ => Response::empty(),
	}
}

impl FixtureObject {
	fn fetch_owner(&self) -> Option<Fixture> {
		let owner = self.owner().ok()?;
		if owner.is_none() {
			log_object_failure(
				None,
				"identify_object",
				"invalid_object",
				"fetch reached an unnamed object",
			);
		}
		owner
	}

	fn owner(&self) -> Result<Option<Fixture>, UnknownFixture> {
		self
			.state
			.id()
			.name()
			.as_deref()
			.map(|name| {
				Fixture::from_object_name(name).inspect_err(|_| {
					log_event(&ObjectFailureEvent {
						event: "worker_experiment.object_failed",
						fixture: None,
						object_name: Some(name),
						operation: "identify_object",
						kind: "invalid_object",
						cause: "object has an unrecognized fixture name",
					});
				})
			})
			.transpose()
	}

	async fn refresh(
		&self,
		storage: &Storage,
		records: &Records,
		requested: Fixture,
	) -> Result<(), ObjectFailure> {
		if let Some(deadline) = records.refresh_deadline()? {
			if deadline > now_ms() {
				return schedule_refresh(storage, deadline).await;
			}
		}
		let result = run_refresh(&self.env, requested).await;
		let deadline = now_ms().saturating_add(OUTCOME_LIFETIME_MS);
		records.write_outcome(result, deadline)?;
		schedule_refresh(storage, deadline).await
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
	if records.read_job(Some(owner))?.is_none() {
		check_owner(owner, requested)?;
		records.write(Record::Job, &RefreshJob::ReadFixture(requested))?;
	}
	if let Some(cached) = records.read_outcome()? {
		if let Err(failure) = ensure_alarm(storage, alarm_running, cached.refresh_after_ms).await {
			record_object_failure(Some(owner), &failure);
		}
		return Ok(cached.for_read(owner));
	}
	ensure_alarm(
		storage,
		alarm_running,
		now_ms().saturating_add(ALARM_DELAY_MS),
	)
	.await?;
	Ok(ReadReply::Pending)
}

fn now_ms() -> i64 {
	i64::try_from(Date::now().as_millis()).unwrap_or(i64::MAX)
}

async fn ensure_alarm(
	storage: &Storage,
	running: &Cell<bool>,
	deadline: i64,
) -> Result<(), ObjectFailure> {
	let scheduled = storage
		.get_alarm()
		.await
		.map_err(|cause| StorageOperation::ReadAlarm.failure(cause))?;
	if scheduled.is_none() && !running.get() {
		schedule_refresh(storage, deadline).await?;
	}
	Ok(())
}

async fn schedule_refresh(storage: &Storage, deadline: i64) -> Result<(), ObjectFailure> {
	storage
		.set_alarm(deadline.saturating_sub(now_ms()).max(0))
		.await
		.map_err(ObjectFailure::ScheduleAlarm)
}

#[derive(Serialize)]
struct StaleValueEvent {
	event: &'static str,
	fixture: Fixture,
	operation: &'static str,
	reason: &'static str,
}

struct CachedOutcome {
	payload: String,
	refresh_after_ms: i64,
	state: CacheState,
}

impl CachedOutcome {
	fn for_read(self, fixture: Fixture) -> ReadReply {
		if self.state == CacheState::Stale {
			log_event(&StaleValueEvent {
				event: "worker_experiment.stale_value_served",
				fixture,
				operation: "read_fixture",
				reason: "refresh_failed",
			});
		}
		ReadReply::Complete(self.payload)
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CacheState {
	Value,
	Failed,
	Stale,
}

impl CacheState {
	const fn name(self) -> &'static str {
		match self {
			Self::Value => "value",
			Self::Failed => "failed",
			Self::Stale => "stale",
		}
	}
}

impl std::str::FromStr for CacheState {
	type Err = ObjectFailure;

	fn from_str(state: &str) -> Result<Self, Self::Err> {
		[Self::Value, Self::Failed, Self::Stale]
			.into_iter()
			.find(|entry| entry.name() == state)
			.ok_or(ObjectFailure::InvalidCacheState)
	}
}

#[derive(Clone, Copy, Debug)]
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
			) STRICT;
			CREATE TABLE IF NOT EXISTS fixture_cache (
				id INTEGER PRIMARY KEY CHECK (id = 1),
				payload TEXT NOT NULL CHECK (json_valid(payload)),
				refresh_after_ms INTEGER NOT NULL,
				state TEXT NOT NULL CHECK (state IN ('value', 'failed', 'stale'))
			) STRICT;
			INSERT INTO fixture_cache (id, payload, refresh_after_ms, state)
			SELECT 1, payload, 0, json_extract(payload, '$.result.outcome')
			FROM fixture_records
			WHERE name = 'outcome' AND NOT EXISTS (SELECT 1 FROM fixture_cache)
			AND json_extract(payload, '$.state') = 'complete'
			AND json_extract(payload, '$.result.outcome') IN ('value', 'failed')",
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

	fn read_job(&self, owner: Option<Fixture>) -> Result<Option<Fixture>, ObjectFailure> {
		let Some(payload) = self.read(Record::Job)? else {
			return Ok(None);
		};
		let RefreshJob::ReadFixture(requested) =
			serde_json::from_str(&payload).map_err(ObjectFailure::DecodeJob)?;
		if let Some(owner) = owner {
			check_owner(owner, requested)?;
		}
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
	fn read_outcome(&self) -> Result<Option<CachedOutcome>, ObjectFailure> {
		let Some(row) = self
			.outcome_row("SELECT payload, refresh_after_ms, state FROM fixture_cache WHERE id = 1")?
		else {
			return Ok(None);
		};
		let mut values = row.into_iter();
		match (values.next(), values.next(), values.next()) {
			(
				Some(SqlStorageValue::String(payload)),
				Some(SqlStorageValue::Integer(refresh_after_ms)),
				Some(SqlStorageValue::String(state)),
			) => Ok(Some(CachedOutcome {
				payload,
				refresh_after_ms,
				state: state.parse()?,
			})),
			_ => Err(ObjectFailure::RecordType(Record::Outcome)),
		}
	}

	fn refresh_deadline(&self) -> Result<Option<i64>, ObjectFailure> {
		match self.outcome_row("SELECT refresh_after_ms FROM fixture_cache WHERE id = 1")? {
			None => Ok(None),
			Some(row) => match row.into_iter().next() {
				Some(SqlStorageValue::Integer(deadline)) => Ok(Some(deadline)),
				_ => Err(ObjectFailure::RecordType(Record::Outcome)),
			},
		}
	}

	fn outcome_row(&self, query: &str) -> Result<Option<Vec<SqlStorageValue>>, ObjectFailure> {
		let on_error = |cause| StorageOperation::Read(Record::Outcome).failure(cause);
		self
			.0
			.exec(query, None)
			.map_err(on_error)?
			.raw()
			.next()
			.transpose()
			.map_err(on_error)
	}

	fn write_outcome(&self, outcome: RefreshOutcome, deadline: i64) -> Result<(), ObjectFailure> {
		let on_error = |cause| StorageOperation::Write(Record::Outcome).failure(cause);
		let state = match outcome {
			RefreshOutcome::Value { .. } => CacheState::Value,
			RefreshOutcome::Failed { .. } => {
				if self.retain_value(deadline)? {
					return Ok(());
				}
				CacheState::Failed
			}
		};
		let payload = serde_json::to_string(&ObjectReply::from(outcome)).map_err(|cause| {
			ObjectFailure::Encode {
				record: Record::Outcome,
				cause,
			}
		})?;
		self
			.0
			.exec(
				"INSERT INTO fixture_cache (id, payload, refresh_after_ms, state) VALUES (1, ?, ?, ?)
			 ON CONFLICT(id) DO UPDATE SET payload = excluded.payload,
			 refresh_after_ms = excluded.refresh_after_ms, state = excluded.state",
				vec![payload.into(), deadline.into(), state.name().into()],
			)
			.map_err(on_error)?;
		Ok(())
	}

	fn retain_value(&self, deadline: i64) -> Result<bool, ObjectFailure> {
		let update = self
			.0
			.exec(
				"UPDATE fixture_cache SET state = 'stale', refresh_after_ms = ?
			 WHERE id = 1 AND state IN ('value', 'stale')",
				vec![deadline.into()],
			)
			.map_err(|cause| StorageOperation::Write(Record::Outcome).failure(cause))?;
		Ok(update.rows_written() != 0)
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
		ObjectReply::StorageUnavailable
		| ObjectReply::AlarmSchedulingFailed
		| ObjectReply::ObjectCallFailed => 503,
		ObjectReply::StoredRecordInvalid => 500,
	};
	Response::from_json(&reply).map(|response| response.with_status(status))
}

fn log_and_reply(fixture: Fixture, failure: &ObjectFailure) -> worker::Result<Response> {
	record_object_failure(Some(fixture), failure);
	reply(failure.reply())
}

mod address {
	use fixture_experiment_route::Fixture;
	use std::net::IpAddr;
	use url::Url;

	use super::FixtureSetupFailure;

	/// The checked URL is only constructed from a numeric HTTP loopback origin.
	pub(super) struct LoopbackUrl(Url);

	impl LoopbackUrl {
		/// Appends the fixture path to a checked loopback origin.
		///
		/// # Errors
		///
		/// Returns a setup failure when the origin cannot be parsed or violates
		/// the loopback HTTP restriction.
		pub(super) fn for_fixture(origin: &str, fixture: Fixture) -> Result<Self, FixtureSetupFailure> {
			let mut url = Url::parse(origin).map_err(FixtureSetupFailure::ParseOrigin)?;
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

		/// Returns the checked request URL for Workers Fetch.
		pub(super) fn as_str(&self) -> &str {
			self.0.as_str()
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum FixtureSetupFailure {
	#[error("EXPERIMENT_FIXTURE_ORIGIN binding is missing or unreadable")]
	Binding(#[source] worker::Error),
	#[error("fixture origin is not a URL")]
	ParseOrigin(#[source] url::ParseError),
	#[error("fixture origin must be a valid loopback HTTP URL")]
	InvalidLoopbackUrl,
	#[error("Workers could not initialize the request cancellation controller")]
	CancellationController(#[source] worker::Error),
}

fn prepare_refresh(
	env: &Env,
	fixture: Fixture,
) -> Result<(LoopbackUrl, AbortController), FixtureSetupFailure> {
	let origin = env
		.var("EXPERIMENT_FIXTURE_ORIGIN")
		.map_err(FixtureSetupFailure::Binding)?;
	let url = LoopbackUrl::for_fixture(&origin.to_string(), fixture)?;
	let controller = AbortController::new()
		.map_err(|cause| FixtureSetupFailure::CancellationController(cause.into()))?;
	Ok((url, controller))
}

async fn run_refresh(env: &Env, fixture: Fixture) -> RefreshOutcome {
	let (url, controller) = match prepare_refresh(env, fixture) {
		Ok(prepared) => prepared,
		Err(error) => return failed_refresh(fixture, RefreshFailure::FixtureSetup, &error),
	};
	let source = FixtureSource(AbortSignal::from(controller.signal()));
	let operation = fixture_operation(source, FixtureAdapter);
	let transfer = pin!(operation.perform(url));
	let deadline = pin!(Delay::from(REFRESH_DEADLINE));
	match select(transfer, deadline).await {
		Either::Left((Ok(value), _)) => RefreshOutcome::Value { value },
		Either::Left((Err(error), _)) => failed_refresh(fixture, error.failure, &error.cause),
		Either::Right(((), _)) => {
			controller.abort();
			log_event(&RefreshFailureEvent::timeout(fixture));
			RefreshOutcome::Failed {
				failure: RefreshFailure::Timeout,
			}
		}
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

struct FixtureSource(AbortSignal);

impl Source<LoopbackUrl> for FixtureSource {
	type Response = Response;
	type Error = SourceFailure;

	fn retrieve<'a>(
		&'a self,
		url: &'a LoopbackUrl,
	) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(fetch_fixture(url, &self.0))
	}
}

/// The SDK wrapper supplies Trifecta's `Send` bound with an unchecked unsafe
/// impl. These futures must stay on the single-threaded Worker executor.
/// The SDK response already satisfies `Send`.
fn fetch_fixture<'a>(
	url: &'a LoopbackUrl,
	signal: &'a AbortSignal,
) -> SendFuture<impl Future<Output = Result<Response, SourceFailure>> + 'a> {
	SendFuture::new(async move {
		let mut init = RequestInit::new();
		init.with_redirect(RequestRedirect::Manual);
		let request = Request::new_with_init(url.as_str(), &init).map_err(SourceFailure::Fetch)?;
		let response = Fetch::Request(request)
			.send_with_signal(signal)
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

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug, thiserror::Error)]
enum ObjectFailure {
	#[error("fixture storage operation {operation:?} failed")]
	Storage {
		operation: StorageOperation,
		#[source]
		cause: worker::Error,
	},
	#[error("could not schedule the fixture alarm")]
	ScheduleAlarm(#[source] worker::Error),
	#[error("could not decode the stored fixture job")]
	DecodeJob(#[source] serde_json::Error),
	#[error("stored {0:?} record has unexpected column types")]
	RecordType(Record),
	#[error("stored outcome has an unrecognized cache state")]
	InvalidCacheState,
	#[error("stored job for {0:?} is in another fixture's object")]
	JobFixtureMismatch(Fixture),
	#[error("could not encode the fixture {record:?} record")]
	Encode {
		record: Record,
		#[source]
		cause: serde_json::Error,
	},
}

impl ObjectFailure {
	const fn reply(&self) -> ObjectReply {
		match self {
			Self::Storage { .. } | Self::Encode { .. } => ObjectReply::StorageUnavailable,
			Self::ScheduleAlarm(_) => ObjectReply::AlarmSchedulingFailed,
			Self::DecodeJob(_)
			| Self::RecordType(_)
			| Self::InvalidCacheState
			| Self::JobFixtureMismatch(_) => ObjectReply::StoredRecordInvalid,
		}
	}
}

#[derive(Serialize)]
struct ObjectFailureEvent<'a> {
	event: &'static str,
	fixture: Option<Fixture>,
	#[serde(skip_serializing_if = "Option::is_none")]
	object_name: Option<&'a str>,
	operation: &'static str,
	kind: &'static str,
	cause: &'a str,
}

/// Records the fixture and operation before reporting a failed reply or
/// propagating a storage error to the alarm runtime for retry.
fn record_object_failure(fixture: Option<Fixture>, failure: &ObjectFailure) {
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
		ObjectFailure::RecordType(record) => log_invalid_record(fixture, *record, failure),
		ObjectFailure::InvalidCacheState => log_invalid_record(fixture, Record::Outcome, failure),
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

fn log_invalid_record(fixture: Option<Fixture>, record: Record, failure: &ObjectFailure) {
	log_object_failure(
		fixture,
		StorageOperation::Read(record).name(),
		"decode_record",
		&failure.to_string(),
	);
}

fn log_object_failure(
	fixture: Option<Fixture>,
	operation: &'static str,
	kind: &'static str,
	cause: &str,
) {
	log_event(&ObjectFailureEvent {
		event: "worker_experiment.object_failed",
		fixture,
		object_name: None,
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
