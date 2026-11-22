//! Shared fixture route for the native and Worker experiments in #452.
//! Both entry points prepare requests with their own HTTP clients and use the
//! same Trifecta operation and completed response. The Durable Object's `fetch`
//! handler produces [`ObjectReply`] for the Worker to forward. Its `alarm`
//! reads a [`RefreshJob`] and persists the operation's [`RefreshOutcome`].

use std::{error::Error as StdError, marker::PhantomData, str::FromStr};

use bytes::{Bytes, BytesMut};
use futures_util::{future, stream, TryStreamExt};
use serde::{Deserialize, Serialize};
use trifecta::{
	body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	operation::{Error as OperationError, ErrorKind, Operation},
	pipeline::Id,
	source::Source,
	transform::Transform,
	uncached::Pipeline,
};

/// Which response the experiment requests from the loopback fixture server.
/// Each fixture has its own Durable Object and owns its job, alarm and result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fixture {
	/// An HTTP 200 with a JSON unsigned integer as its body.
	Value,
	/// An HTTP 302 pointing to a separate fixture path.
	Redirect,
}

/// The input does not identify either fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unknown fixture")]
pub struct UnknownFixture;

impl Fixture {
	const ALL: [Self; 2] = [Self::Value, Self::Redirect];

	const fn names(self) -> (&'static str, &'static str) {
		match self {
			Self::Value => ("value", "fixture-value"),
			Self::Redirect => ("redirect", "fixture-redirect"),
		}
	}

	/// The stable path segment and serialized fixture parameter.
	#[must_use]
	pub const fn segment(self) -> &'static str {
		self.names().0
	}

	/// The stable name used to select the fixture's Durable Object.
	#[must_use]
	pub const fn object_name(self) -> &'static str {
		self.names().1
	}

	/// Identifies a fixture by its Durable Object name.
	///
	/// # Errors
	///
	/// Returns [`UnknownFixture`] for other names.
	pub fn from_object_name(name: &str) -> Result<Self, UnknownFixture> {
		Self::ALL
			.into_iter()
			.find(|fixture| fixture.object_name() == name)
			.ok_or(UnknownFixture)
	}

	/// Parses the full route path shared by both entry points.
	///
	/// # Errors
	///
	/// Returns [`UnknownFixture`] unless the path is `/experiment/{fixture}`
	/// with a known fixture segment.
	pub fn from_route_path(path: &str) -> Result<Self, UnknownFixture> {
		path
			.strip_prefix("/experiment/")
			.ok_or(UnknownFixture)?
			.parse()
	}
}

impl FromStr for Fixture {
	type Err = UnknownFixture;

	/// Parses the exact fixture segment used in the route and in storage.
	fn from_str(segment: &str) -> Result<Self, Self::Err> {
		Self::ALL
			.into_iter()
			.find(|fixture| fixture.segment() == segment)
			.ok_or(UnknownFixture)
	}
}

impl Serialize for Fixture {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(self.segment())
	}
}

impl<'de> Deserialize<'de> for Fixture {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		String::deserialize(deserializer)?
			.parse()
			.map_err(serde::de::Error::custom)
	}
}

/// The operation and parameters persisted before scheduling a refresh.
/// The alarm uses the stored parameters to prepare the request. The object name
/// serves as a diagnostic identity and a check that the job reached its owner.
/// A mismatch is an invalid stored record.
/// The SQLite storage format is `{"name":"read_fixture","parameters":"value"}`
/// or the same object with `"redirect"` as its parameters.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "name", content = "parameters")]
pub enum RefreshJob {
	/// Fetch and decode the selected fixture using the draft Trifecta pipeline.
	/// The job belongs to the Durable Object named for that fixture.
	#[serde(rename = "read_fixture")]
	ReadFixture(Fixture),
}

/// A completed fixture attempt. The Worker encodes this inside a completed
/// [`ObjectReply`] before persisting the ready-to-serve JSON in SQLite.
/// The nested outcome format is `{"outcome":"value","value":42}` for a value,
/// or `{"outcome":"failed","failure":{"kind":"http_status","status":302}}`
/// for a failure, with the inner `kind` and fields defined by [`RefreshFailure`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome")]
pub enum RefreshOutcome {
	/// The integer decoded from a successful upstream response.
	#[serde(rename = "value")]
	Value {
		/// The unsigned integer sent by the fixture.
		value: u32,
	},
	/// A completed attempt whose upstream or body could not be used.
	#[serde(rename = "failed")]
	Failed {
		/// The stage or HTTP response that ruled out producing a value.
		failure: RefreshFailure,
	},
}

/// An HTTP status in `100..=199` or `300..=599`.
/// Construction checks these ranges before the status can be serialized.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct NonSuccessStatus(u16);

/// The supplied number is a success status or is outside the HTTP status range.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{0} is not a non-success HTTP status")]
pub struct InvalidNonSuccessStatus(u16);

impl TryFrom<u16> for NonSuccessStatus {
	type Error = InvalidNonSuccessStatus;

	/// Checks an HTTP response status before storing it as a failure.
	///
	/// # Errors
	///
	/// Returns [`InvalidNonSuccessStatus`] for success statuses and numbers
	/// outside `100..=599`.
	fn try_from(status: u16) -> Result<Self, Self::Error> {
		match status {
			100..=199 | 300..=599 => Ok(Self(status)),
			_ => Err(InvalidNonSuccessStatus(status)),
		}
	}
}

impl std::fmt::Display for NonSuccessStatus {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(formatter)
	}
}

/// A refresh failure returned to the caller as data.
/// The alarm logs the underlying cause before persisting this classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum RefreshFailure {
	/// The transport failed before it yielded a response.
	#[serde(rename = "fetch")]
	Fetch,
	/// Fetching the headers and reading the body exceeded the refresh deadline.
	#[serde(rename = "timeout")]
	Timeout,
	/// The transport returned a response without a valid HTTP status, such as
	/// status zero on an opaque redirect.
	#[serde(rename = "invalid_status")]
	InvalidStatus,
	/// The local fixture address was missing or could not be used as a loopback URL.
	#[serde(rename = "fixture_setup")]
	FixtureSetup,
	/// The upstream sent a response outside the success range.
	#[serde(rename = "http_status")]
	HttpStatus {
		/// The original HTTP status, including 302 when the fixture redirects.
		status: NonSuccessStatus,
	},
	/// The upstream response body could not be read.
	#[serde(rename = "read_body")]
	ReadBody,
	/// The response body was not a JSON unsigned integer in the `u32` range.
	#[serde(rename = "decode")]
	Decode,
	/// Trifecta reported a stage or cause outside this uncached pipeline's contract.
	#[serde(rename = "pipeline")]
	Pipeline,
}

/// The JSON shape for completed responses from either entry point and for
/// interim or storage-related replies from the Durable Object. The Worker
/// forwards this body as sent by the object.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state")]
pub enum ObjectReply {
	/// The fixture's alarm is scheduled or running. If a later read finds an
	/// unfinished job with neither a scheduled nor a running alarm, the object
	/// re-arms the same job and returns `Pending` only after scheduling succeeds.
	#[serde(rename = "pending")]
	Pending,
	/// A completed attempt for the fixture that owns this object.
	#[serde(rename = "complete")]
	Complete {
		/// The last good value, or a failed refresh when no good value exists.
		result: RefreshOutcome,
	},
	/// The object could not read or write SQLite storage.
	#[serde(rename = "storage_unavailable")]
	StorageUnavailable,
	/// Storage is accessible, but the runtime rejected the alarm schedule.
	#[serde(rename = "alarm_scheduling_failed")]
	AlarmSchedulingFailed,
	/// The Worker could not resolve or call the fixture's Durable Object.
	#[serde(rename = "object_call_failed")]
	ObjectCallFailed,
	/// The stored job could not be decoded or belonged to a different fixture.
	#[serde(rename = "stored_record_invalid")]
	StoredRecordInvalid,
}

impl From<RefreshOutcome> for ObjectReply {
	fn from(result: RefreshOutcome) -> Self {
		Self::Complete { result }
	}
}

/// Typed causes that a transport returns from the Trifecta source stage.
#[derive(Debug, thiserror::Error)]
pub enum FixtureSourceError<E> {
	/// The runtime failed to construct or execute the upstream request.
	#[error("fixture transport failed")]
	Fetch(#[source] E),
	/// The upstream returned a valid non-success HTTP status.
	#[error("fixture returned HTTP {0}")]
	HttpStatus(NonSuccessStatus),
	/// The upstream response did not have a valid HTTP status.
	#[error("fixture transport returned an invalid HTTP status")]
	InvalidStatus(#[source] InvalidNonSuccessStatus),
}

/// Moves a buffered fixture response into a one-chunk Trifecta body.
/// The stream owns the bytes and has no runtime or transport handles.
pub fn buffered_fixture_body(bytes: Bytes) -> BodyWithMetadata<()> {
	let chunks = stream::once(future::ready(Ok(bytes)));
	BodyWithMetadata::new(Body::new(Box::pin(chunks)), ())
}

/// The shared conversion failed while consuming or parsing the adapted body.
#[derive(Debug, thiserror::Error)]
enum TransformFailure {
	#[error("failed to consume the adapted body")]
	ReadBody(#[source] BodyReadError),
	#[error("fixture body was not a JSON unsigned integer")]
	Decode(#[source] serde_json::Error),
}

struct FixtureTransform;

impl<P> Transform<P, (), u32> for FixtureTransform {
	type Error = TransformFailure;

	fn transform<'a>(
		&'a self,
		_request: &'a P,
		body: BodyWithMetadata<()>,
	) -> BoxFuture<'a, Result<u32, Self::Error>> {
		Box::pin(decode_fixture(body))
	}
}

async fn decode_fixture(body: BodyWithMetadata<()>) -> Result<u32, TransformFailure> {
	let mut chunks = body.into_parts().0.into_stream();
	let first = chunks
		.try_next()
		.await
		.map_err(TransformFailure::ReadBody)?
		.unwrap_or_default();
	let Some(second) = chunks
		.try_next()
		.await
		.map_err(TransformFailure::ReadBody)?
	else {
		return serde_json::from_slice(&first).map_err(TransformFailure::Decode);
	};
	let mut bytes = BytesMut::from(first.as_ref());
	bytes.extend_from_slice(&second);
	while let Some(chunk) = chunks
		.try_next()
		.await
		.map_err(TransformFailure::ReadBody)?
	{
		bytes.extend_from_slice(&chunk);
	}
	serde_json::from_slice(&bytes).map_err(TransformFailure::Decode)
}

/// The uncached fixture pipeline with its transport error type retained for
/// classification after Trifecta erases the concrete cause.
pub struct FixtureOperation<P, E> {
	operation: Operation<P, u32>,
	transport_error: PhantomData<fn() -> E>,
}

/// Assembles the same fixture pipeline from either transport. The source's
/// error type determines how this operation later recovers its concrete cause.
pub fn fixture_operation<P, S, A, E>(source: S, adapter: A) -> FixtureOperation<P, E>
where
	P: Send + Sync + 'static,
	S: Source<P, Error = FixtureSourceError<E>>,
	A: AdaptResponse<S::Response, Metadata = ()>,
	E: StdError + Send + Sync + 'static,
{
	let pipeline = Pipeline::new(source, adapter, FixtureTransform);
	FixtureOperation {
		operation: Operation::new(pipeline.into_handle(Id::new("read_fixture"))),
		transport_error: PhantomData,
	}
}

/// A classified fixture failure and the original Trifecta diagnostic for logs.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct FailedRefresh {
	/// The failure safe to include in a response or persist in SQLite.
	pub failure: RefreshFailure,
	/// The original pipeline error with its concrete error source.
	#[source]
	pub cause: OperationError,
}

impl<P, E> FixtureOperation<P, E>
where
	P: Send + Sync + 'static,
	E: StdError + Send + Sync + 'static,
{
	/// Performs the shared operation and returns the decoded fixture integer.
	///
	/// # Errors
	///
	/// Returns the classified source, body, or conversion failure with its cause.
	pub async fn perform(&self, request: P) -> Result<u32, FailedRefresh> {
		self
			.operation
			.perform(request)
			.await
			.map(trifecta::operation::Outcome::into_value)
			.map_err(|cause| FailedRefresh {
				failure: Self::classify(&cause),
				cause,
			})
	}

	fn classify(error: &OperationError) -> RefreshFailure {
		match error.kind() {
			ErrorKind::Source => match error
				.source()
				.and_then(|cause| cause.downcast_ref::<FixtureSourceError<E>>())
			{
				Some(FixtureSourceError::Fetch(_)) => RefreshFailure::Fetch,
				Some(FixtureSourceError::HttpStatus(status)) => {
					RefreshFailure::HttpStatus { status: *status }
				}
				Some(FixtureSourceError::InvalidStatus(_)) => RefreshFailure::InvalidStatus,
				None => RefreshFailure::Pipeline,
			},
			ErrorKind::ResponseAdaptation => RefreshFailure::ReadBody,
			ErrorKind::Transformation => match error
				.source()
				.and_then(|cause| cause.downcast_ref::<TransformFailure>())
			{
				Some(TransformFailure::ReadBody(_)) => RefreshFailure::ReadBody,
				Some(TransformFailure::Decode(_)) => RefreshFailure::Decode,
				None => RefreshFailure::Pipeline,
			},
			_ => RefreshFailure::Pipeline,
		}
	}
}

/// The shared structured refresh failure event for native and Worker logs.
#[derive(Debug, Serialize)]
pub struct RefreshFailureEvent<'a> {
	event: &'static str,
	fixture: Fixture,
	operation: &'static str,
	failure: RefreshFailure,
	cause: &'a str,
}

impl<'a> RefreshFailureEvent<'a> {
	/// Uses a diagnostic prepared by the entry point with sensitive data removed.
	#[must_use]
	pub const fn new(fixture: Fixture, failure: RefreshFailure, cause: &'a str) -> Self {
		Self {
			event: "fixture_experiment.refresh_failed",
			fixture,
			operation: "read_fixture",
			failure,
			cause,
		}
	}
}
