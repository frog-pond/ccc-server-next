//! Pure cache policy for reads, refreshes, and storage failures.
//!
//! The cached pipeline supplies metadata, timestamps, and refresh outcomes.
//! The decision describes what to serve, write, fetch, or schedule. The
//! pipeline applies it and reports failures. Each source defines its freshness
//! and stale serving policy and supplies any epoch or per-value lifetime.

#![deny(missing_docs)]
// Temporary while the draft signatures are under review.
#![allow(
	unused_variables,
	clippy::missing_panics_doc,
	clippy::needless_pass_by_value
)]

use std::{error::Error, fmt, num::NonZeroU32};

/// A caller-supplied point in time, in whole milliseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(pub i64);

/// A nonnegative interval at whole-millisecond precision.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Milliseconds(pub u64);

/// An opaque source-defined epoch. Only equality matters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Epoch(pub String);

/// Freshness and stale serving rules for one source.
///
/// A source computes its current epoch and any per-value lifetime. Those enter
/// the decision separately as read and refresh inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Policy {
	/// Default freshness interval starting at the fetch's start time.
	pub fresh_for: Milliseconds,
	/// Additional time after freshness when a read can request an alarm
	/// refresh and answer immediately, if the stored epoch matches.
	pub stale_while_revalidate: Milliseconds,
	/// Age limit applied when the answer would otherwise be stale after an
	/// upstream failure.
	pub stale_on_error: StaleOnError,
}

/// How old the last good value may be when an upstream failure affects a read.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StaleOnError {
	/// Serve the last good value at any age.
	#[default]
	Unlimited,
	/// Serve the last good value up to and including this age, measured from
	/// the start of its successful fetch.
	MaxAge(Milliseconds),
}

impl Policy {
	/// Set the freshness and stale-while-revalidate windows with an
	/// unlimited stale-on-error policy.
	#[must_use]
	pub const fn new(fresh_for: Milliseconds, stale_while_revalidate: Milliseconds) -> Self {
		todo!()
	}
}

/// The metadata a backend keeps for one resource, beside its optional value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry<E> {
	/// The read time recorded at creation or by the last throttled touch.
	pub last_read: Timestamp,
	/// Metadata present exactly when the backend holds a last good value.
	pub stored: Option<StoredValue>,
	/// The unsuccessful refresh streak since the last success, if any.
	pub failure: Option<Failure<E>>,
}

/// Metadata for the last successful fetch, stored together with its value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredValue {
	/// When the fetch began, even if it completed after an epoch change.
	pub fetched_at: Timestamp,
	/// The source's epoch at fetch start, absent for a source without epochs.
	pub epoch: Option<Epoch>,
	/// The freshness interval the source computed from this value, if it
	/// supplies a per-value override. Zero is a valid override.
	pub fresh_for: Option<Milliseconds>,
}

/// A recorded refresh failure. Backoff can expire while this record persists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure<E> {
	/// Number of consecutive failed refreshes, including this failure.
	pub consecutive: NonZeroU32,
	/// Reads suppress foreground fetches and immediate alarms before this time.
	pub backoff_until: Timestamp,
	/// The typed cause of the most recent failed refresh.
	pub last_error: E,
}

/// What just happened for the resource.
#[derive(Debug)]
pub enum Event<'a, E> {
	/// A caller wants an answer. Source registration has already been checked.
	Read {
		/// The entry as read from cache, absent on a cold miss.
		entry: Option<&'a Entry<E>>,
		/// The epoch at read time, absent if the source has no epoch rule.
		epoch: Option<&'a Epoch>,
	},
	/// A scheduled refresh fired and has been consumed.
	Alarm {
		/// The entry as read from cache, absent if it has been removed.
		entry: Option<&'a Entry<E>>,
	},
	/// The refresh owner has an outcome to apply. Pass the latest entry so a
	/// read that touched it during the fetch counts toward its idle deadline.
	RefreshFinished {
		/// The entry immediately before applying the refresh outcome.
		entry: &'a Entry<E>,
		/// Success metadata, the recorded failure, or an abort before loading.
		outcome: RefreshOutcome<'a, E>,
	},
	/// A read of cache storage failed. The pipeline reports the underlying error.
	CacheReadFailed(ReadFailurePolicy),
	/// A write to cache storage failed. The pipeline reports the underlying error.
	CacheWriteFailed(WriteFailurePolicy),
}

/// Facts supplied by the refresh owner. The value stays with the caller.
#[derive(Debug)]
pub enum RefreshOutcome<'a, E> {
	/// Replace the stored value and reset the failure streak.
	Succeeded(StoredValue),
	/// Keep the stored value and record a failed refresh.
	Failed {
		/// When the fetch started. Backoff is measured from this point.
		started_at: Timestamp,
		/// The typed refresh failure.
		error: &'a E,
		/// The longest parsed upstream delay, or zero when none was requested.
		/// The decision caps this at one day.
		retry_after: Milliseconds,
	},
	/// Loading stopped before an outcome could be recorded. Answer waiting
	/// readers from the existing entry and leave scheduling alone.
	Aborted,
}

/// How the pipeline responds to a cache storage read failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadFailurePolicy {
	/// Return the cache error.
	Propagate,
	/// Fetch directly from the source without relying on cached data.
	FetchFromSource,
}

/// How the pipeline responds to a cache storage write failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteFailurePolicy {
	/// Return the cache error.
	Propagate,
	/// Keep the answer produced by the pipeline despite the failed write.
	ReturnAnswer,
}

/// Cache actions for the supplied event.
#[derive(Debug, Eq, PartialEq)]
pub enum Decision<'a, E> {
	/// Changes initiated by a read.
	Read {
		/// When present, insert a missing entry or update an existing entry's
		/// last-read time. Apply this before carrying out the read action.
		record_read_at: Option<Timestamp>,
		/// Whether the caller can answer or must wait for a refresh.
		action: ReadAction<'a, E>,
	},
	/// Refresh from the consumed alarm. The pipeline starts or joins the
	/// fetch through singleflight. There is no read to answer here.
	RefreshFromAlarm,
	/// A consumed alarm had no active entry to refresh.
	LeaveDormant,
	/// Apply a completed refresh once and give any waiting readers the recorded
	/// outcome.
	RefreshFinished(RefreshDecision<'a, E>),
	/// Propagate the cache error from this event.
	PropagateCacheFailure,
	/// On cache read failure, fetch directly from the source and wait for it.
	FetchFromSource,
	/// On cache write failure, keep the existing answer.
	ReturnAnswer,
}

/// What a read does after any last-read write.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadAction<'a, E> {
	/// Return the stored value within its freshness interval and epoch.
	Fresh {
		/// The start time of the fetch that populated the value.
		fetched_at: Timestamp,
	},
	/// Return the stored value within the stale window before any failure,
	/// and request an immediate refresh through the alarm.
	Stale {
		/// The start time of the fetch that populated the value.
		fetched_at: Timestamp,
		/// Replace the pending alarm with this deadline.
		refresh_at: Timestamp,
	},
	/// A previous failure affects the answer. The source's stale-on-error
	/// policy determines whether to serve the stored value.
	AfterFailure {
		/// The stored value or error to return immediately.
		answer: FailureAnswer<'a, E>,
		/// Replace the pending alarm with this deadline when within the
		/// stale window and past backoff. Otherwise leave scheduling alone.
		refresh_at: Option<Timestamp>,
	},
	/// Start or join the refresh through singleflight and wait for its outcome.
	WaitForRefresh,
}

/// What a read can return when it relies on upstream failure handling.
#[derive(Debug, Eq, PartialEq)]
pub enum FailureAnswer<'a, E> {
	/// Serve the last good value as stale due to an upstream failure.
	ServeStored {
		/// The start time of the successful fetch, possibly in another epoch.
		fetched_at: Timestamp,
	},
	/// No stored value can be served under the source's policy.
	Error(ReadError<'a, E>),
}

/// Why a read affected by upstream failure could not return a value.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadError<'a, E> {
	/// A refresh failed and there is no usable stored value.
	UpstreamFailed(&'a E),
	/// No usable value or recorded upstream cause exists after an aborted load.
	UpstreamUnavailable,
}

impl<E: fmt::Display> fmt::Display for ReadError<'_, E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!()
	}
}

impl<E: Error + 'static> Error for ReadError<'_, E> {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		todo!()
	}
}

/// What to write and what to give readers after the owner completes a refresh.
#[derive(Debug, Eq, PartialEq)]
pub enum RefreshDecision<'a, E> {
	/// Store the new value and metadata, reset the failure streak, and answer
	/// any waiting reads with the new value marked fresh.
	StoreValue {
		/// Metadata to store with the new value and use in the fresh answer.
		stored: StoredValue,
		/// Replace or clear the pending alarm after the write.
		alarm: NextAlarm,
	},
	/// Record a failed refresh without replacing the last good value or its
	/// metadata. The failure can also be reported from this outcome.
	RecordFailure {
		/// Failure record to persist alongside the existing value, if any.
		failure: Failure<&'a E>,
		/// How to answer waiting reads using the last good value or error.
		answer: FailureAnswer<'a, E>,
		/// Replace or clear the pending alarm after the write.
		alarm: NextAlarm,
	},
	/// Leave the entry and alarm alone. The pipeline handles the abort itself.
	LeaveUnchanged {
		/// How to answer waiting reads using the existing entry.
		answer: FailureAnswer<'a, E>,
	},
}

/// What to do with the pending alarm after recording a refresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NextAlarm {
	/// Replace the pending alarm. A time already due requests immediate work.
	Set(Timestamp),
	/// Clear the pending alarm because the resource has gone idle.
	Clear,
}

/// Decide how to handle one cache event.
///
/// Supply the time at which the read, alarm, or refresh scheduling decision is
/// made. A completed refresh separately carries the time at which it began.
/// The pipeline applies storage and alarm writes, handles singleflight and
/// registration, and reports failures. This function only describes actions.
#[must_use]
pub fn decide<E: Error>(policy: Policy, now: Timestamp, event: Event<'_, E>) -> Decision<'_, E> {
	todo!()
}
