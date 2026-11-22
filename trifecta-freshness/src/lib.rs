//! Pure cache policy for reads, refreshes, and storage failures.
//!
//! The cached pipeline in `trifecta` supplies metadata, times, and refresh
//! outcomes. This crate computes decisions and entry updates. It calls no
//! external services or pipeline code. Its rules cover reads, background
//! refresh, backoff, epochs, value-dependent lifetimes, and idle resources
//! in `docs/reference/cache-semantics.md` §§4–9.

#![deny(missing_docs)]
// Temporary while the draft signatures are under review.
#![allow(
	unused_variables,
	clippy::missing_panics_doc,
	clippy::needless_pass_by_value
)]

use std::{error::Error, num::NonZeroU32, time::Duration};

/// A caller-supplied point in time, in whole milliseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(pub i64);

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
	pub fresh_for: Duration,
	/// Additional time after freshness when a read can request a background
	/// refresh and answer immediately, if the stored epoch matches. This is
	/// the [reference's `staleIfError`][window], as described in
	/// `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_while_revalidate: Duration,
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
	MaxAge(Duration),
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
	pub fresh_for: Option<Duration>,
}

/// A recorded refresh failure. Backoff can expire while this record persists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure<E> {
	/// Number of consecutive failed refreshes, including this failure.
	pub consecutive: NonZeroU32,
	/// Reads suppress foreground fetches and immediate background refreshes before this time.
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
		retry_after: Duration,
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
pub enum ReadDecision {
	/// Insert an entry without a stored value or failure, then start or join
	/// a refresh and wait for its outcome. A failed insert still leads to
	/// the refresh under [`WriteFailurePolicy::ReturnAnswer`].
	ColdMiss {
		/// The last-read time to set on the new entry.
		insert_at: Timestamp,
	},
	/// Carry out the action for an entry that already exists.
	ExistingEntry {
		/// Update the entry's last-read time when this timestamp is present.
		touch_at: Option<Timestamp>,
		/// Continue with this action after the touch, including when the touch
		/// fails under [`WriteFailurePolicy::ReturnAnswer`].
		action: ReadAction,
	},
}

/// Whether a scheduled background refresh that has come due should run.
#[derive(Debug, Eq, PartialEq)]
pub enum DueRefreshDecision {
	/// Start or join a refresh through singleflight.
	Run,
	/// The entry is missing or idle. Schedule no new refresh.
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
	/// and request an immediate refresh in the background.
	Stale {
		/// The start time of the fetch that populated the value.
		fetched_at: Timestamp,
		/// Replace the pending background refresh with this deadline.
		refresh_at: Timestamp,
	},
	/// A previous failure affects the answer. The source's stale-on-error
	/// policy determines whether to serve the stored value.
	AfterFailure {
		/// The stored value or error to return immediately.
		answer: FailureAnswer<StoredRefreshError>,
		/// Replace the pending background refresh with this deadline when within the
		/// stale window and past backoff. Otherwise leave scheduling alone.
		refresh_at: Option<Timestamp>,
	},
	/// Start or join the refresh through singleflight and wait for its outcome.
	WaitForRefresh,
}

/// What a read can return when it relies on upstream failure handling.
#[derive(Debug, Eq, PartialEq)]
pub enum FailureAnswer<Cause> {
	/// Serve the last good value as stale due to an upstream failure.
	ServeStored {
		/// The start time of the successful fetch, possibly in another epoch.
		fetched_at: Timestamp,
	},
	/// No stored value can be served under the source's policy.
	Error(Cause),
}

/// Report the error from the refresh that just failed.
#[derive(Debug, Eq, PartialEq)]
pub struct CurrentRefreshError;

/// Report the error recorded on the existing entry.
#[derive(Debug, Eq, PartialEq)]
pub struct StoredRefreshError;

/// The error to report after an abort leaves no usable stored value.
#[derive(Debug, Eq, PartialEq)]
pub enum AbortedRefreshError {
	/// Report the stored error from an earlier failed refresh.
	Stored(StoredRefreshError),
	/// No failure was recorded. Show exactly `upstream unavailable`.
	///
	/// This is the fallback at the [reference's read return][fallback].
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L130
	UpstreamUnavailable,
}

/// What to write and what to give readers after a refresh.
#[derive(Debug, Eq, PartialEq)]
pub enum RefreshDecision {
	/// Write the entry from [`entry_after_success`] and mark the new value fresh
	/// for waiting readers. After the write, use [`decide_next_background_refresh`].
	StoreValue {
		/// Metadata for the loaded value to store and serve.
		stored: StoredValue,
	},
	/// A refresh succeeded without an entry to update. Give waiting readers
	/// the new value marked fresh and leave storage and scheduling alone.
	ReturnFresh {
		/// The start time of the successful fetch.
		fetched_at: Timestamp,
	},
	/// Write the entry from [`entry_after_failure`]. After the write, use
	/// [`decide_next_background_refresh`].
	RecordFailure {
		/// The computed update to persist with the current error.
		update: FailureUpdate,
		/// Serve a stored value or report the current refresh's error.
		answer: FailureAnswer<CurrentRefreshError>,
	},
	/// A refresh failed without an entry to update. Report the current
	/// refresh's error and leave storage and scheduling alone.
	ReturnCurrentError,
	/// A refresh aborted. Leave storage and scheduling alone.
	Aborted {
		/// Serve a stored value, report a stored error, or report unavailable
		/// when no failure was recorded.
		answer: FailureAnswer<AbortedRefreshError>,
	},
}

/// Failure metadata computed at refresh completion for the next cache write.
#[derive(Debug, Eq, PartialEq)]
pub struct FailureUpdate {
	/// The new count of consecutive failed refreshes.
	pub consecutive: NonZeroU32,
	/// Reads suppress foreground fetches and immediate refreshes before this time.
	pub backoff_until: Timestamp,
}

/// Build the entry to write after a successful refresh.
///
/// Keep the last-read time, store the supplied metadata, and clear any
/// failure. After writing it with the value, use the written entry to
/// [compute the next background refresh](decide_next_background_refresh).
#[must_use]
pub fn entry_after_success<E>(entry: Entry<E>, stored: StoredValue) -> Entry<E> {
	todo!()
}

/// Build the entry to write after a failed refresh.
///
/// Keep the last-read time and the last successful value's metadata. Store
/// the computed failure update with the caller's current error. After writing
/// it, use the written entry to
/// [compute the next background refresh](decide_next_background_refresh).
#[must_use]
pub fn entry_after_failure<E>(entry: Entry<E>, update: FailureUpdate, error: E) -> Entry<E> {
	todo!()
}

/// When to refresh a resource in the background after recording a load outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NextBackgroundRefresh {
	/// Replace the pending refresh with this deadline. A deadline already due
	/// requests immediate work.
	ScheduleAt(Timestamp),
	/// Cancel the pending refresh because the resource has gone idle.
	Stop,
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

/// Decide whether a scheduled background refresh that has come due should run.
///
/// Pass the entry as read when the refresh comes due and the time of that read.
#[must_use]
pub fn decide_due_refresh<E: Error>(
	now: Timestamp,
	entry: Option<&Entry<E>>,
) -> DueRefreshDecision {
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

/// Decide when to next refresh in the background after writing a value or failure.
///
/// Pass the entry as written and a fresh timestamp sampled after the write.
/// Cancel the pending refresh if the entry has gone idle. Otherwise, schedule at the end
/// of an active backoff or at the value's expiration. If there is no stored
/// value and backoff has elapsed, schedule one default freshness interval
/// after the supplied time.
#[must_use]
pub fn decide_next_background_refresh<E: Error>(
	policy: Policy,
	now: Timestamp,
	entry: &Entry<E>,
) -> NextBackgroundRefresh {
	todo!()
}
