//! Storage and HTTP response types for the Rust Worker experiment in #452.

use serde::{Deserialize, Serialize};

/// Which response the experiment requests from the loopback fixture server.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fixture {
	/// An HTTP 200 with a JSON unsigned integer as its body.
	Value,
	/// An HTTP 302 pointing to a separate fixture path.
	Redirect,
}

/// The operation and parameters persisted before scheduling a refresh.
/// An alarm decodes this record and prepares a new request from its binding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "name", content = "parameters", rename_all = "snake_case")]
pub enum RefreshJob {
	/// Fetch and decode the selected fixture using the draft Trifecta pipeline.
	ReadFixture(Fixture),
}

/// The outcome of a refresh, persisted in the SQLite-backed Durable Object.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RefreshOutcome {
	/// The integer decoded from a successful upstream response.
	Value {
		/// The unsigned integer sent by the fixture.
		value: u32,
	},
	/// A completed attempt whose upstream or body could not be used.
	Failed {
		/// The stage or HTTP response that ruled out producing a value.
		failure: RefreshFailure,
	},
}

/// A refresh failure returned to the caller as data.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefreshFailure {
	/// The runtime failed to send the request or receive a response.
	Fetch,
	/// The upstream sent a response outside the success range.
	HttpStatus {
		/// The original HTTP status, including 302 when the fixture redirects.
		status: u16,
	},
	/// The upstream response body could not be read.
	ReadBody,
	/// The response body was not a JSON unsigned integer in the `u32` range.
	Decode,
}

/// The value returned across the Durable Object call as an HTTP JSON body.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ObjectReply {
	/// The object has scheduled the first refresh and its result is pending.
	Pending,
	/// A completed attempt recorded in storage.
	Complete {
		/// The result returned by the last completed refresh.
		result: RefreshOutcome,
	},
	/// The object could not access storage or schedule its alarm.
	StorageUnavailable,
}
