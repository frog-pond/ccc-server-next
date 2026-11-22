//! Shared fixture route for the native and Worker experiments in #452.
//! Both entry points prepare requests with their own HTTP clients and use the
//! same Trifecta operation and completed response. The Durable Object's `fetch`
//! handler produces [`ObjectReply`] for the Worker to forward. Its `alarm`
//! reads a [`RefreshJob`] and persists the operation's [`RefreshOutcome`].

// The shared pipeline bodies follow the draft signature review.
#![allow(dead_code)]

use bytes::Bytes;
use futures_util::{future, stream};
use serde::{Deserialize, Serialize};
use trifecta::{
	body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	operation::{Error as OperationError, Operation},
	source::Source,
	transform::Transform,
};

/// Which response the experiment requests from the loopback fixture server.
/// Each fixture has its own Durable Object and owns its job, alarm and result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Fixture {
	/// An HTTP 200 with a JSON unsigned integer as its body.
	/// Uses the Durable Object named `fixture-value`.
	#[serde(rename = "value")]
	Value,
	/// An HTTP 302 pointing to a separate fixture path.
	/// Uses the Durable Object named `fixture-redirect`.
	#[serde(rename = "redirect")]
	Redirect,
}

impl Fixture {
	/// Parses the last path segment of `/experiment/{fixture}`.
	/// Returns `None` for segments other than `value` and `redirect`.
	#[must_use]
	pub fn from_path_segment(segment: &str) -> Option<Self> {
		match segment {
			"value" => Some(Self::Value),
			"redirect" => Some(Self::Redirect),
			_ => None,
		}
	}

	/// Returns the stable name used by the Worker to select the fixture's object.
	#[must_use]
	pub const fn object_name(self) -> &'static str {
		match self {
			Self::Value => "fixture-value",
			Self::Redirect => "fixture-redirect",
		}
	}

	/// Identifies the fixture from the name supplied to the Durable Object.
	/// Returns `None` for an object outside this route's two known names.
	#[must_use]
	pub fn from_object_name(name: &str) -> Option<Self> {
		[Self::Value, Self::Redirect]
			.into_iter()
			.find(|fixture| fixture.object_name() == name)
	}

	/// The HTTP path to request from the loopback fixture server.
	#[must_use]
	pub const fn upstream_path(self) -> &'static str {
		match self {
			Self::Value => "/value",
			Self::Redirect => "/redirect",
		}
	}
}

/// The operation and parameters persisted before scheduling a refresh.
/// An alarm decodes this record and prepares a new request from its binding.
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

/// The outcome of a refresh, persisted in the SQLite-backed Durable Object.
/// The SQLite storage format is `{"outcome":"value","value":42}` for a value,
/// or `{"outcome":"failed","failure":{"kind":"http_status","status":302}}`
/// for a failure, with the inner `kind` and fields defined by [`RefreshFailure`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
/// Construction and deserialization both check these ranges.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "u16")]
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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum RefreshFailure {
	/// The transport failed before it yielded a response.
	#[serde(rename = "fetch")]
	Fetch,
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
		/// The result returned by the last completed refresh.
		result: RefreshOutcome,
	},
	/// The object could not read or write SQLite storage.
	#[serde(rename = "storage_unavailable")]
	StorageUnavailable,
	/// Storage is accessible, but the runtime rejected the alarm schedule.
	#[serde(rename = "alarm_scheduling_failed")]
	AlarmSchedulingFailed,
	/// A stored job or result was read but could not be decoded.
	#[serde(rename = "stored_record_invalid")]
	StoredRecordInvalid,
}

impl From<RefreshOutcome> for ObjectReply {
	fn from(result: RefreshOutcome) -> Self {
		Self::Complete { result }
	}
}

impl ObjectReply {
	/// Gives both entry points the status for the serialized reply. A completed
	/// attempt is delivered as 200 even when the embedded refresh failed.
	#[must_use]
	pub const fn http_status(self) -> u16 {
		match self {
			Self::Complete { .. } => 200,
			Self::Pending => 202,
			Self::StorageUnavailable | Self::AlarmSchedulingFailed => 503,
			Self::StoredRecordInvalid => 500,
		}
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
		_body: BodyWithMetadata<()>,
	) -> BoxFuture<'a, Result<u32, Self::Error>> {
		Box::pin(async { todo!("decode the integer from the adapted body") })
	}
}

/// Assembles the same uncached fixture operation from either entry point's
/// transport source and adapter. Both supply a successful body with no metadata.
pub fn fixture_operation<P, S, A>(_source: S, _adapter: A) -> Operation<P, u32>
where
	P: Send + Sync + 'static,
	S: Source<P>,
	A: AdaptResponse<S::Response, Metadata = ()>,
{
	todo!("compose the source and adapter with FixtureTransform")
}

/// Builds the outcome shared by both entry points. `E` is the transport's
/// underlying error, such as `worker::Error` or `reqwest::Error`.
///
/// `Source` downcasts to [`FixtureSourceError`] to distinguish `Fetch`,
/// `InvalidStatus` and `HttpStatus`. For `HttpStatus`, the source captures the
/// checked status from its original HTTP response. `ResponseAdaptation` maps to
/// `ReadBody`. `Transformation` downcasts to select `ReadBody` or `Decode`.
/// An unexpected stage or cause maps to `Pipeline`. The caller still owns the
/// original error and can log it before it stores or returns the outcome.
#[must_use]
pub fn classify_refresh_failure<E>(_error: &OperationError) -> RefreshFailure
where
	E: std::error::Error + Send + Sync + 'static,
{
	todo!("classify the stage and downcast its cause where needed")
}
