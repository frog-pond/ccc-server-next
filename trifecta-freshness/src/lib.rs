//! Pure cache policy for reads, refreshes, and storage failures.
//!
//! The cached pipeline supplies metadata, timestamps, and refresh outcomes.
//! The decisions describe what to serve, write, fetch, or schedule. The
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
	/// refresh and answer immediately, if the stored epoch matches. This is
	/// the [reference's `staleIfError`][window], as described in
	/// `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_while_revalidate: Milliseconds,
	/// Age limit applied when the answer would otherwise be stale after an
	/// upstream failure. The [reference returns the stored value at any age][fallback].
	/// The optional limit was agreed to in [#439].
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L129
	/// [#439]: https://github.com/frog-pond/ccc-server-next/issues/439
	pub stale_on_error: StaleOnError,
}

/// How old the last good value may be when an upstream failure affects a read.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StaleOnError {
	/// Serve the last good value at any age, matching the reference.
	#[default]
	Unlimited,
	/// Serve the last good value up to and including this age, measured from
	/// the start of its successful fetch. This is the opt-in limit from #439.
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

/// Facts supplied by the refresh owner. The loaded value or actual error
/// stays with the caller.
#[derive(Debug)]
pub enum RefreshOutcome {
	/// A value was loaded successfully with this metadata.
	Succeeded(StoredValue),
	/// A refresh failed.
	Failed {
		/// When the fetch started. Backoff is measured from this point.
		started_at: Timestamp,
		/// The longest parsed upstream delay, or zero when none was requested.
		/// The decision caps this at one day.
		retry_after: Milliseconds,
	},
	/// Loading stopped before an outcome could be recorded. Answer waiting
	/// readers from the existing entry, if any, and leave scheduling alone.
	Aborted,
}

/// What the pipeline does after reporting a cache storage read failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadFailurePolicy {
	/// Return the cache error to the caller.
	Propagate,
	/// Fetch directly from the source, wait for it, and return that outcome.
	FetchFromSource,
}

/// What the pipeline does after reporting a cache storage write failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteFailurePolicy {
	/// Stop and return the cache error to the caller.
	Propagate,
	/// Keep the outcome of the operation. If the failed write was an insert
	/// or touch before a read action, continue with that same action. If it
	/// was a write after a refresh, return the answer without rescheduling.
	ReturnAnswer,
}

/// The last-read write and pending action for a read.
#[derive(Debug, Eq, PartialEq)]
pub struct ReadDecision {
	/// When present, insert a missing entry or update an existing entry's
	/// last-read time before carrying out the action. On a write failure,
	/// apply the pipeline's [write failure policy](WriteFailurePolicy).
	pub record_read_at: Option<Timestamp>,
	/// Whether to answer or wait for a refresh. This action is still available
	/// if the last-read write fails under [`WriteFailurePolicy::ReturnAnswer`].
	pub action: ReadAction,
}

/// What to do when a scheduled alarm has been consumed.
#[derive(Debug, Eq, PartialEq)]
pub enum AlarmDecision {
	/// Start or join a refresh through singleflight.
	Refresh,
	/// The entry is missing or idle. Leave it without an alarm.
	LeaveDormant,
}

/// What a read does after any last-read write.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadAction {
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
		answer: FailureAnswer,
		/// Replace the pending alarm with this deadline when within the
		/// stale window and past backoff. Otherwise leave scheduling alone.
		refresh_at: Option<Timestamp>,
	},
	/// Start or join the refresh through singleflight and wait for its outcome.
	WaitForRefresh,
}

/// What a read can return when it relies on upstream failure handling.
#[derive(Debug, Eq, PartialEq)]
pub enum FailureAnswer {
	/// Serve the last good value as stale due to an upstream failure.
	ServeStored {
		/// The start time of the successful fetch, possibly in another epoch.
		fetched_at: Timestamp,
	},
	/// No stored value can be served under the source's policy.
	Error(ReadError),
}

/// Why a read affected by upstream failure could not return a value.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadError {
	/// A refresh failed and there is no usable stored value. Use the error
	/// from this refresh if it just failed, or the stored error when answering
	/// during backoff or after an aborted refresh.
	UpstreamFailed,
	/// No usable value or recorded upstream cause exists after an aborted load.
	UpstreamUnavailable,
}

impl fmt::Display for ReadError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!()
	}
}

impl Error for ReadError {}

/// What to write and what to give readers after a refresh.
#[derive(Debug, Eq, PartialEq)]
pub enum RefreshDecision {
	/// Store the new value and metadata, reset the failure streak, and answer
	/// waiting reads with the new value marked fresh. After a successful write,
	/// use [`decide_next_alarm`] with the entry as written.
	StoreValue {
		/// Metadata to store with the new value and use in the fresh answer.
		stored: StoredValue,
	},
	/// A refresh succeeded without an entry to update. Give waiting readers
	/// the new value marked fresh, with no storage or alarm change.
	ReturnFresh {
		/// The start time of the successful fetch.
		fetched_at: Timestamp,
	},
	/// Record a failed refresh without replacing the last good value or its
	/// metadata. The caller supplies and reports the error. After a successful
	/// write, use [`decide_next_alarm`] with the entry as written.
	RecordFailure {
		/// The new count of consecutive failed refreshes.
		consecutive: NonZeroU32,
		/// Reads suppress foreground fetches and immediate alarms before this time.
		backoff_until: Timestamp,
		/// How to answer waiting reads using the stored value or current error.
		answer: FailureAnswer,
	},
	/// Leave storage and the alarm alone when a refresh was aborted or when
	/// it failed without an entry to update.
	LeaveUnchanged {
		/// How to answer waiting reads from the existing entry or error outcome.
		answer: FailureAnswer,
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

/// Decide how to answer a read for a registered source.
///
/// Pass the cache entry if present, the time of the read, and the source's
/// current epoch if it has an epoch rule. The pipeline applies the last-read
/// write and carries out the pending action.
#[must_use]
pub fn decide_read<E: Error>(
	policy: Policy,
	now: Timestamp,
	entry: Option<&Entry<E>>,
	epoch: Option<&Epoch>,
) -> ReadDecision {
	todo!()
}

/// Decide whether a consumed alarm should request a refresh.
///
/// Pass the entry as read when the alarm fires and the time of that read.
#[must_use]
pub fn decide_alarm<E: Error>(now: Timestamp, entry: Option<&Entry<E>>) -> AlarmDecision {
	todo!()
}

/// Decide what to write and how to answer waiting readers when a refresh ends.
///
/// Pass the latest entry so a read that touched it during the refresh counts.
/// If the entry is missing, leave storage and scheduling alone. Waiting
/// readers get the newly loaded value on success, the current error on
/// failure, or an unavailable error on abort.
#[must_use]
pub fn decide_refresh_finished<E: Error>(
	policy: Policy,
	now: Timestamp,
	entry: Option<&Entry<E>>,
	outcome: RefreshOutcome,
) -> RefreshDecision {
	todo!()
}

/// Decide the next alarm after writing either a successful value or a failure.
///
/// Pass the entry as written and a fresh timestamp sampled after the write.
/// Clear the alarm if the entry has gone idle. Otherwise, schedule at the end
/// of an active backoff or at the value's expiration. If there is no stored
/// value and backoff has elapsed, schedule one default freshness interval
/// after the supplied time.
#[must_use]
pub fn decide_next_alarm<E: Error>(policy: Policy, now: Timestamp, entry: &Entry<E>) -> NextAlarm {
	todo!()
}
