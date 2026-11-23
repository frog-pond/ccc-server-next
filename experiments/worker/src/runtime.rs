//! Workers invokes these entry points. The Durable Object stores the fixture job
//! and completed reply in SQLite and invokes the shared Trifecta operation from
//! its alarm.

use std::{
	cell::Cell,
	error::Error,
	future::Future,
	num::NonZeroU32,
	pin::pin,
	rc::Rc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

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
use serde::{Deserialize, Serialize};
use trifecta::{
	body::{AdaptResponse, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};
use trifecta_freshness::{
	failure_after, record_read, CachedValueMetadata, Entry, Failure, FreshnessPolicy, ReadAction,
};
use worker::{
	durable_object, event, send::SendFuture, wasm_bindgen, web_sys::AbortController, AbortSignal,
	Context, Date, Delay, DurableObject, Env, Fetch, Headers, Request, RequestInit, RequestRedirect,
	Response, SqlStorage, SqlStorageValue, State, Storage,
};

use address::LoopbackUrl;
use input_gate::InputGate;

const FIXTURES: &str = "FIXTURES";
// Use a short fresh period so the local run can observe the next alarm.
const FIXTURE_POLICY: FreshnessPolicy<SystemTime> = FreshnessPolicy {
	fresh_for: Duration::from_secs(5),
	stale_window: Duration::from_secs(60),
	refresh_ahead: Duration::ZERO,
	serve_cached_after_failure: None,
	period_end: None,
};

type FixtureEntry = Entry<SystemTime, ()>;

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
	if request.inner().method() != "GET" {
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
				operation: EventOperation::Call(failure.operation),
				kind: EventKind::ObjectCall,
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

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ObjectCallOperation {
	ResolveBinding,
	GetByName,
	FetchWithRequest,
}

impl ObjectCallOperation {
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
	input_gate: InputGate,
	env: Env,
	// get_alarm() can return None while the alarm handler is running.
	alarm_running: Rc<Cell<bool>>,
	// A failed SQLite touch must still be visible to the alarm on this instance.
	unwritten_read: Rc<Cell<Option<SystemTime>>>,
}

impl DurableObject for FixtureObject {
	fn new(state: State, env: Env) -> Self {
		let (state, input_gate) = InputGate::from_state(state);
		Self {
			state,
			input_gate,
			env,
			alarm_running: Rc::new(Cell::new(false)),
			unwritten_read: Rc::new(Cell::new(None)),
		}
	}

	async fn fetch(&self, request: Request) -> worker::Result<Response> {
		let Some(owner) = self.fetch_owner() else {
			return reply(ObjectReply::InvalidObject);
		};
		let Ok(requested) = Fixture::from_route_path(&request.path()) else {
			return Response::empty().map(|response| response.with_status(404));
		};
		let storage = self.state.storage();
		let alarm_running = Rc::clone(&self.alarm_running);
		let unwritten_read = Rc::clone(&self.unwritten_read);
		// Keep the storage/alarm lookup and re-arm together. Return routine
		// failures as values so they do not fail the concurrency gate.
		let result = self
			.input_gate
			.run(async move {
				read_or_schedule(&storage, owner, requested, &alarm_running, &unwritten_read).await
			})
			.await;
		match result {
			Ok(Ok(ReadReply::Complete(payload))) => completed_response(payload),
			Ok(Ok(ReadReply::Pending)) => reply(ObjectReply::Pending),
			Ok(Err(failure)) => log_and_reply(owner, &failure),
			Err(error) => log_and_reply(owner, &StorageOperation::ConcurrencyGate.failure(error)),
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
				EventOperation::IdentifyObject,
				EventKind::InvalidObject,
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
						operation: EventOperation::IdentifyObject,
						kind: EventKind::InvalidObject,
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
		let Some(mut stored) = self.read_alarm_entry(records)? else {
			return Ok(());
		};
		let started_at = worker_time()?;
		if FIXTURE_POLICY.may_refresh(started_at, Some(&stored.policy_entry())) {
			let outcome = run_refresh(&self.env, requested).await;
			records.write_outcome(outcome, &stored, started_at, requested)?;
			stored = self
				.read_alarm_entry(records)?
				.ok_or(ObjectFailure::MissingEntry)?;
		}
		if let Some(deadline) = FIXTURE_POLICY.next_refresh_hint(worker_time()?, &stored.policy_entry())
		{
			schedule_refresh(storage, deadline).await?;
		}
		Ok(())
	}

	fn read_alarm_entry(&self, records: &Records) -> Result<Option<StoredEntry>, ObjectFailure> {
		Ok(StoredEntry::apply_read(
			records.read_entry()?,
			self.unwritten_read.get(),
		))
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
	unwritten_read: &Cell<Option<SystemTime>>,
) -> Result<ReadReply, ObjectFailure> {
	check_owner(owner, requested)?;
	let records = Records::open(storage.sql())?;
	let now = worker_time()?;
	let stored = load_for_read(&records, owner, now, unwritten_read)?;
	let entry = stored.policy_entry();
	let action = FIXTURE_POLICY.on_read(now, Some(&entry));
	let response = stored.for_read(owner, action)?;
	if let Err(failure) = ensure_alarm(
		storage,
		alarm_running,
		now,
		&entry,
		requests_refresh(action),
	)
	.await
	{
		match response {
			ReadReply::Pending => return Err(failure),
			ReadReply::Complete(_) => record_object_failure(Some(owner), &failure),
		}
	}
	Ok(response)
}

fn load_for_read(
	records: &Records,
	fixture: Fixture,
	now: SystemTime,
	unwritten_read: &Cell<Option<SystemTime>>,
) -> Result<StoredEntry, ObjectFailure> {
	if records.read_job(Some(fixture))?.is_none() {
		records.write_job(RefreshJob::ReadFixture(fixture))?;
	}
	let stored = StoredEntry::apply_read(records.read_entry()?, unwritten_read.get());
	let entry = stored.as_ref().map(StoredEntry::policy_entry);
	let last_read = record_read(now, entry.as_ref());
	let stored = StoredEntry::apply_read(stored, last_read).ok_or(ObjectFailure::MissingEntry)?;
	if let Some(last_read) = last_read {
		unwritten_read.set(Some(last_read));
	}
	if let Some(last_read) = unwritten_read.get() {
		match records.write_read_time(last_read) {
			Ok(()) => unwritten_read.set(None),
			Err(failure) => record_object_failure(Some(fixture), &failure),
		}
	}
	Ok(stored)
}

fn requests_refresh(action: ReadAction<'_, SystemTime, ()>) -> bool {
	matches!(
		action,
		ReadAction::FetchWithoutFallback
			| ReadAction::FetchWithStaleFallback { .. }
			| ReadAction::RequestRefresh { .. }
			| ReadAction::RequestRefreshAfterFailure { .. }
			| ReadAction::ServeFreshAndRequestRefresh { .. }
	)
}

fn worker_time() -> Result<SystemTime, ObjectFailure> {
	time_from_millis(Date::now().as_millis()).ok_or(ObjectFailure::InvalidClock)
}

fn time_from_millis(millis: u64) -> Option<SystemTime> {
	UNIX_EPOCH.checked_add(Duration::from_millis(millis))
}

fn storage_time(millis: i64) -> Result<SystemTime, ObjectFailure> {
	u64::try_from(millis)
		.ok()
		.and_then(time_from_millis)
		.ok_or(ObjectFailure::RecordType(Record::Outcome))
}

fn storage_millis(time: SystemTime) -> Result<i64, ObjectFailure> {
	time
		.duration_since(UNIX_EPOCH)
		.ok()
		.and_then(|since_epoch| i64::try_from(since_epoch.as_millis()).ok())
		.ok_or(ObjectFailure::InvalidClock)
}

async fn ensure_alarm(
	storage: &Storage,
	running: &Cell<bool>,
	now: SystemTime,
	entry: &FixtureEntry,
	request_refresh: bool,
) -> Result<(), ObjectFailure> {
	if running.get() {
		return Ok(());
	}
	let scheduled = storage
		.get_alarm()
		.await
		.map_err(|cause| StorageOperation::ReadAlarm.failure(cause))?;
	let deadline = if request_refresh {
		let now_ms = storage_millis(now)?;
		scheduled
			.is_none_or(|millis| millis > now_ms)
			.then_some(now)
	} else {
		scheduled
			.is_none()
			.then(|| FIXTURE_POLICY.next_refresh_hint(now, entry))
			.flatten()
	};
	if let Some(deadline) = deadline {
		schedule_refresh(storage, deadline).await?;
	}
	Ok(())
}

async fn schedule_refresh(storage: &Storage, deadline: SystemTime) -> Result<(), ObjectFailure> {
	let delay = deadline.duration_since(worker_time()?).unwrap_or_default();
	storage
		.set_alarm(delay)
		.await
		.map_err(ObjectFailure::ScheduleAlarm)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum StaleReason {
	RefreshFailed,
}

#[derive(Serialize)]
struct StaleValueEvent {
	event: &'static str,
	fixture: Fixture,
	operation: &'static str,
	reason: StaleReason,
}

struct StoredValue {
	metadata: CachedValueMetadata<SystemTime>,
	payload: String,
}

struct StoredEntry {
	last_read: SystemTime,
	value: Option<StoredValue>,
	failure: Option<Failure<SystemTime, String>>,
}

impl StoredEntry {
	fn apply_read(stored: Option<Self>, read: Option<SystemTime>) -> Option<Self> {
		match (stored, read) {
			(Some(mut stored), Some(read)) => {
				stored.last_read = stored.last_read.max(read);
				Some(stored)
			}
			(None, Some(last_read)) => Some(Self {
				last_read,
				value: None,
				failure: None,
			}),
			(stored, None) => stored,
		}
	}

	// The policy needs the failure to decide what to serve. The encoded
	// failure reply stays alongside it and moves to the response if selected.
	fn policy_entry(&self) -> FixtureEntry {
		Entry {
			last_read: self.last_read,
			cached: self.value.as_ref().map(|value| value.metadata),
			failure: self.failure.as_ref().map(|failure| Failure {
				consecutive_failures: failure.consecutive_failures,
				backoff_until: failure.backoff_until,
				last_error: (),
			}),
		}
	}

	fn for_read(
		self,
		fixture: Fixture,
		action: ReadAction<'_, SystemTime, ()>,
	) -> Result<ReadReply, ObjectFailure> {
		match action {
			ReadAction::ServeFresh { .. }
			| ReadAction::ServeFreshAndRequestRefresh { .. }
			| ReadAction::RequestRefresh { .. } => self.complete_value(fixture, None),
			ReadAction::RequestRefreshAfterFailure { .. }
			| ReadAction::ServeCachedAfterFailure { .. } => {
				self.complete_value(fixture, Some(StaleReason::RefreshFailed))
			}
			ReadAction::ReportCachedError(()) => self
				.failure
				.map(|failure| ReadReply::Complete(failure.last_error))
				.ok_or(ObjectFailure::MissingEntry),
			// The experiment has no waiting readers. A later read applies the
			// policy again after the alarm records its success or failure.
			ReadAction::FetchWithStaleFallback { .. } | ReadAction::FetchWithoutFallback => {
				Ok(ReadReply::Pending)
			}
		}
	}

	fn complete_value(
		self,
		fixture: Fixture,
		stale_reason: Option<StaleReason>,
	) -> Result<ReadReply, ObjectFailure> {
		let value = self.value.ok_or(ObjectFailure::MissingEntry)?;
		if let Some(reason) = stale_reason {
			log_event(&StaleValueEvent {
				event: "worker_experiment.stale_value_served",
				fixture,
				operation: "read_fixture",
				reason,
			});
		}
		Ok(ReadReply::Complete(value.payload))
	}
}

#[derive(Clone, Copy, Debug)]
enum Record {
	Job,
	Outcome,
}

/// SQLite's version of the freshness entry. The value and failure payloads
/// are the completed reply JSON, each encoded once when its alarm finishes.
#[derive(Deserialize)]
struct EntryRow {
	last_read_ms: i64,
	value_started_ms: Option<i64>,
	value_payload: Option<String>,
	failure_count: Option<NonZeroU32>,
	backoff_until_ms: Option<i64>,
	failure_payload: Option<String>,
}

impl EntryRow {
	fn decode(self) -> Result<StoredEntry, ObjectFailure> {
		let value = match (self.value_started_ms, self.value_payload) {
			(Some(started), Some(payload)) => Some(StoredValue {
				metadata: CachedValueMetadata::new(storage_time(started)?, None, None)?,
				payload,
			}),
			(None, None) => None,
			_ => return Err(ObjectFailure::RecordType(Record::Outcome)),
		};
		let failure = match (
			self.failure_count,
			self.backoff_until_ms,
			self.failure_payload,
		) {
			(Some(consecutive_failures), Some(until), Some(last_error)) => Some(Failure {
				consecutive_failures,
				backoff_until: storage_time(until)?,
				last_error,
			}),
			(None, None, None) => None,
			_ => return Err(ObjectFailure::RecordType(Record::Outcome)),
		};
		Ok(StoredEntry {
			last_read: storage_time(self.last_read_ms)?,
			value,
			failure,
		})
	}
}

struct Records(SqlStorage);

impl Records {
	fn open(sql: SqlStorage) -> Result<Self, ObjectFailure> {
		sql
			.exec(include_str!("fixture_storage.sql"), None)
			.map_err(|cause| StorageOperation::OpenStorage.failure(cause))?;
		Ok(Self(sql))
	}

	fn read_job(&self, owner: Option<Fixture>) -> Result<Option<Fixture>, ObjectFailure> {
		let on_error = |cause| StorageOperation::ReadJob.failure(cause);
		let cursor = self
			.0
			.exec(
				"SELECT payload FROM fixture_records WHERE name = 'job'",
				None,
			)
			.map_err(on_error)?;
		let Some(row) = cursor.raw().next().transpose().map_err(on_error)? else {
			return Ok(None);
		};
		let Some(SqlStorageValue::String(payload)) = row.into_iter().next() else {
			return Err(ObjectFailure::RecordType(Record::Job));
		};
		let RefreshJob::ReadFixture(requested) =
			serde_json::from_str(&payload).map_err(ObjectFailure::DecodeJob)?;
		if let Some(owner) = owner {
			check_owner(owner, requested)?;
		}
		Ok(Some(requested))
	}

	fn write_job(&self, job: RefreshJob) -> Result<(), ObjectFailure> {
		let payload = encode(Record::Job, &job)?;
		self
			.0
			.exec(
				"INSERT INTO fixture_records (name, payload) VALUES ('job', ?)
			 ON CONFLICT(name) DO UPDATE SET payload = excluded.payload",
				vec![payload.into()],
			)
			.map_err(|cause| StorageOperation::WriteJob.failure(cause))?;
		Ok(())
	}

	fn read_entry(&self) -> Result<Option<StoredEntry>, ObjectFailure> {
		let cursor = self
			.0
			.exec(
				"SELECT last_read_ms, value_started_ms, value_payload, failure_count,
			 backoff_until_ms, failure_payload FROM fixture_cache_entry WHERE id = 1",
				None,
			)
			.map_err(|cause| StorageOperation::ReadOutcome.failure(cause))?;
		cursor
			.next::<EntryRow>()
			.next()
			.transpose()
			.map_err(ObjectFailure::DecodeEntry)?
			.map(EntryRow::decode)
			.transpose()
	}

	fn write_read_time(&self, now: SystemTime) -> Result<(), ObjectFailure> {
		self
			.0
			.exec(
				"INSERT INTO fixture_cache_entry (id, last_read_ms) VALUES (1, ?)
			 ON CONFLICT(id) DO UPDATE SET last_read_ms = max(last_read_ms, excluded.last_read_ms)",
				vec![storage_millis(now)?.into()],
			)
			.map_err(|cause| StorageOperation::WriteReadTime.failure(cause))?;
		Ok(())
	}

	fn write_outcome(
		&self,
		outcome: RefreshOutcome,
		stored: &StoredEntry,
		started_at: SystemTime,
		fixture: Fixture,
	) -> Result<(), ObjectFailure> {
		let outcome = match outcome {
			RefreshOutcome::Value { .. } => match FIXTURE_POLICY.cached_after_success(started_at, None) {
				Ok(metadata) => {
					return self.write_value(stored.last_read, metadata, encode_outcome(outcome)?);
				}
				Err(error) => failed_refresh(fixture, RefreshFailure::Pipeline, &error),
			},
			RefreshOutcome::Failed { .. } => outcome,
		};
		self.write_failure(stored, started_at, encode_outcome(outcome)?)
	}

	fn write_value(
		&self,
		last_read: SystemTime,
		cached: CachedValueMetadata<SystemTime>,
		payload: String,
	) -> Result<(), ObjectFailure> {
		// A read may advance during the fetch. Only supply a read time for
		// the insert, which is needed if the first read-time write failed.
		self
			.0
			.exec(
				"INSERT INTO fixture_cache_entry (id, last_read_ms, value_started_ms, value_payload)
			 VALUES (1, ?, ?, ?)
			 ON CONFLICT(id) DO UPDATE SET value_started_ms = excluded.value_started_ms,
			 value_payload = excluded.value_payload, failure_count = NULL,
			 backoff_until_ms = NULL, failure_payload = NULL",
				vec![
					storage_millis(last_read)?.into(),
					storage_millis(cached.fetch_started_at())?.into(),
					payload.into(),
				],
			)
			.map_err(|cause| StorageOperation::WriteOutcome.failure(cause))?;
		Ok(())
	}

	fn write_failure(
		&self,
		stored: &StoredEntry,
		started_at: SystemTime,
		payload: String,
	) -> Result<(), ObjectFailure> {
		let previous_failures = stored
			.failure
			.as_ref()
			.map(|failure| failure.consecutive_failures);
		let failure = failure_after(started_at, previous_failures, payload, Duration::ZERO);
		self.0.exec(
			"INSERT INTO fixture_cache_entry (id, last_read_ms, failure_count, backoff_until_ms, failure_payload)
			 VALUES (1, ?, ?, ?, ?)
			 ON CONFLICT(id) DO UPDATE SET failure_count = excluded.failure_count,
			 backoff_until_ms = excluded.backoff_until_ms, failure_payload = excluded.failure_payload",
			vec![
				storage_millis(stored.last_read)?.into(),
				i64::from(failure.consecutive_failures.get()).into(),
				storage_millis(failure.backoff_until)?.into(),
				failure.last_error.into(),
			],
		).map_err(|cause| StorageOperation::WriteOutcome.failure(cause))?;
		Ok(())
	}
}

fn encode_outcome(outcome: RefreshOutcome) -> Result<String, ObjectFailure> {
	encode(Record::Outcome, &ObjectReply::from(outcome))
}

fn encode(record: Record, value: &impl Serialize) -> Result<String, ObjectFailure> {
	serde_json::to_string(value).map_err(|cause| ObjectFailure::Encode { record, cause })
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
		ObjectReply::StoredRecordInvalid | ObjectReply::InvalidObject => 500,
	};
	Response::from_json(&reply).map(|response| response.with_status(status))
}

fn log_and_reply(fixture: Fixture, failure: &ObjectFailure) -> worker::Result<Response> {
	record_object_failure(Some(fixture), failure);
	reply(failure.reply())
}

mod input_gate;

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

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum StorageOperation {
	OpenStorage,
	ReadJob,
	ReadOutcome,
	WriteJob,
	WriteOutcome,
	WriteReadTime,
	ReadAlarm,
	ConcurrencyGate,
}

impl StorageOperation {
	const fn read(record: Record) -> Self {
		match record {
			Record::Job => Self::ReadJob,
			Record::Outcome => Self::ReadOutcome,
		}
	}

	const fn write(record: Record) -> Self {
		match record {
			Record::Job => Self::WriteJob,
			Record::Outcome => Self::WriteOutcome,
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
	#[error("could not decode the stored freshness entry")]
	DecodeEntry(#[source] worker::Error),
	#[error("the freshness entry is missing its expected record")]
	MissingEntry,
	#[error("the Worker clock cannot be represented in storage")]
	InvalidClock,
	#[error("stored cached value metadata is invalid")]
	InvalidMetadata(#[from] trifecta_freshness::Error),
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
			Self::Storage { .. } | Self::Encode { .. } | Self::InvalidClock => {
				ObjectReply::StorageUnavailable
			}
			Self::ScheduleAlarm(_) => ObjectReply::AlarmSchedulingFailed,
			Self::DecodeJob(_)
			| Self::RecordType(_)
			| Self::DecodeEntry(_)
			| Self::MissingEntry
			| Self::InvalidMetadata(_)
			| Self::JobFixtureMismatch(_) => ObjectReply::StoredRecordInvalid,
		}
	}
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum EventOperation {
	IdentifyObject,
	ScheduleAlarm,
	DecodeJob,
	ValidateJob,
	ReadClock,
	#[serde(untagged)]
	Storage(StorageOperation),
	#[serde(untagged)]
	Call(ObjectCallOperation),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum EventKind {
	ObjectCall,
	InvalidObject,
	Storage,
	ScheduleAlarm,
	DecodeRecord,
	EncodeRecord,
}

#[derive(Serialize)]
struct ObjectFailureEvent<'a> {
	event: &'static str,
	fixture: Option<Fixture>,
	#[serde(skip_serializing_if = "Option::is_none")]
	object_name: Option<&'a str>,
	operation: EventOperation,
	kind: EventKind,
	cause: &'a str,
}

/// Records the fixture and operation before reporting a failed reply or
/// propagating a storage error to the alarm runtime for retry.
fn record_object_failure(fixture: Option<Fixture>, failure: &ObjectFailure) {
	use EventKind as Kind;
	use EventOperation as Operation;

	let log = |operation, kind, cause: &str| log_object_failure(fixture, operation, kind, cause);
	match failure {
		ObjectFailure::Storage { operation, cause } => {
			log(Operation::Storage(*operation), Kind::Storage, cause.into())
		}
		ObjectFailure::ScheduleAlarm(cause) => {
			log(Operation::ScheduleAlarm, Kind::ScheduleAlarm, cause.into())
		}
		ObjectFailure::DecodeJob(cause) => {
			log(Operation::DecodeJob, Kind::DecodeRecord, &cause.to_string())
		}
		ObjectFailure::RecordType(record) => log_invalid_record(fixture, *record, failure),
		ObjectFailure::DecodeEntry(cause) => log(
			Operation::Storage(StorageOperation::ReadOutcome),
			Kind::DecodeRecord,
			cause.into(),
		),
		ObjectFailure::MissingEntry | ObjectFailure::InvalidMetadata(_) => {
			log_invalid_record(fixture, Record::Outcome, failure);
		}
		ObjectFailure::InvalidClock => log(Operation::ReadClock, Kind::Storage, &failure.to_string()),
		ObjectFailure::JobFixtureMismatch(job) => {
			log(Operation::ValidateJob, Kind::DecodeRecord, job.segment());
		}
		ObjectFailure::Encode { record, cause } => log(
			Operation::Storage(StorageOperation::write(*record)),
			Kind::EncodeRecord,
			&cause.to_string(),
		),
	}
}

fn log_invalid_record(fixture: Option<Fixture>, record: Record, failure: &ObjectFailure) {
	log_object_failure(
		fixture,
		EventOperation::Storage(StorageOperation::read(record)),
		EventKind::DecodeRecord,
		&failure.to_string(),
	);
}

fn log_object_failure(
	fixture: Option<Fixture>,
	operation: EventOperation,
	kind: EventKind,
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
