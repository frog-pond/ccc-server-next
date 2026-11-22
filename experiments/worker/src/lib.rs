//! The Rust Worker experiment in #452. The Worker route selects the fixture's
//! Durable Object and forwards its `fetch` response. The object's `fetch`
//! stores a [`RefreshJob`] and schedules its alarm, or reads an existing
//! [`RefreshOutcome`], and returns an [`ObjectReply`] to the route. The object's
//! `alarm` reads the job, runs the refresh, and stores the outcome.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
mod runtime;

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
	/// The runtime could not fetch a response with a valid HTTP status.
	#[serde(rename = "fetch")]
	Fetch,
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

/// The value returned across the Durable Object call as an HTTP JSON body.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state")]
pub enum ObjectReply {
	/// The fixture's job and alarm were accepted, and no outcome has been stored.
	/// An upstream or decoding failure becomes `Complete` once stored. An alarm
	/// that fails to access or write storage can leave this state visible while
	/// the runtime retries, and after the last retry if storage never recovers.
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
