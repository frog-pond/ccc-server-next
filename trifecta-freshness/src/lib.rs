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

/// A read rewrites the last-read time only after more than this interval.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L20).
pub const TOUCH_EVERY: Duration = Duration::from_secs(60 * 10);

/// An entry is idle only when more than this interval has passed since its
/// recorded last read. Both background-refresh decisions use this rule.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L18).
pub const IDLE_AFTER: Duration = Duration::from_secs(2 * 24 * 60 * 60);

/// Base backoff for the first consecutive failure, measured from fetch start.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L15).
pub const MIN_BACKOFF: Duration = Duration::from_secs(30);

/// Upper bound for the exponential part of backoff.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L16).
pub const MAX_BACKOFF: Duration = Duration::from_secs(60 * 60);

/// Longest upstream-requested delay accepted from `Retry-After`.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/conditional.ts#L53).
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// A caller-supplied point in time, in whole milliseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(pub i64);

impl Timestamp {
	/// Add a nonnegative interval to this timestamp.
	///
	/// Round a fractional millisecond up to the next whole millisecond so a
	/// deadline never fires early. Clamp a deadline above the timestamp range
	/// to `Timestamp(i64::MAX)`.
	#[must_use]
	pub fn saturating_add(self, duration: Duration) -> Self {
		todo!()
	}

	/// Return the elapsed time since an earlier timestamp at whole-millisecond
	/// precision. If `earlier` is later than this timestamp, return zero. Even
	/// the difference between the earliest and latest timestamps fits in a
	/// `Duration`.
	#[must_use]
	pub fn saturating_duration_since(self, earlier: Self) -> Duration {
		todo!()
	}
}

/// A source-defined label for the period a value belongs to, such as a campus
/// day. A value with a different label can only be served as an upstream
/// failure fallback. Only equality matters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Epoch(pub String);

/// Freshness and stale serving rules for one source.
///
/// A source computes its current epoch and any per-value lifetime. Those enter
/// the decision separately as read and refresh inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreshnessPolicy {
	/// Default freshness interval starting at the fetch's start time.
	pub fresh_for: Duration,
	/// Additional time after freshness when a read can request a background
	/// refresh and answer immediately, if the stored epoch matches. This is
	/// the [reference's `staleIfError`][window], as described in
	/// `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_window: Duration,
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
	/// If a read needs a fallback and the value is past the limit, report the
	/// stored error during backoff. Once backoff ends, wait for a refresh even
	/// if the value is still in the stale window.
	MaxAge(Duration),
}

/// The metadata a backend keeps for one resource, beside its optional value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry<E> {
	/// Time recorded on insertion or a read more than [`TOUCH_EVERY`] later.
	pub last_read: Timestamp,
	/// Metadata present exactly when the backend holds a last good value.
	pub stored: Option<CachedValueMetadata>,
	/// The unsuccessful refresh streak since the last success, if any.
	pub failure: Option<Failure<E>>,
}

/// Metadata for the last successful fetch, stored together with its value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedValueMetadata {
	/// When the fetch began, even if it completed after an epoch change.
	pub fetch_started_at: Timestamp,
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
	/// Reads are backed off when this is strictly later than the read time.
	pub backoff_until: Timestamp,
	/// The typed cause of the most recent failed refresh.
	pub last_error: E,
}

/// Facts supplied by the refresh owner. The loaded value or actual error
/// stays with the caller.
#[derive(Debug)]
pub enum RefreshOutcome {
	/// A value was loaded successfully with this metadata.
	Succeeded(CachedValueMetadata),
	/// A refresh failed.
	Failed {
		/// When the fetch started. Backoff is measured from this point.
		fetch_started_at: Timestamp,
		/// The longest parsed upstream delay, or zero when none was requested.
		/// Cap it at [`MAX_RETRY_AFTER`] before comparing it with exponential backoff.
		retry_after: Duration,
	},
	/// Loading stopped before an outcome could be recorded. Answer waiting
	/// readers from the existing entry, if any, and leave scheduling alone.
	Aborted,
}

/// What the pipeline does after reporting a cache storage read failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageReadFailurePolicy {
	/// Return the cache error to the caller.
	Propagate,
	/// Fetch directly from the source, wait for it, and return that outcome.
	FetchFromSource,
}

/// What the pipeline does after reporting a cache storage write failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageWriteFailurePolicy {
	/// End the operation and return the cache error to the caller.
	Propagate,
	/// Keep the outcome of the operation. If the failed write was an insert
	/// or touch before a read action, continue with that same action. If it
	/// was a write after a refresh, return the answer without rescheduling.
	ContinueWithoutWrite,
}

/// The last-read write and pending action for a read.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadDecision<'a, E> {
	/// Insert an entry without a stored value or failure, then start or join
	/// a refresh and wait for its outcome. A failed insert still leads to
	/// the refresh under [`StorageWriteFailurePolicy::ContinueWithoutWrite`].
	ColdMiss {
		/// The last-read time to set on the new entry.
		insert_at: Timestamp,
	},
	/// Carry out the action for an entry that already exists.
	ExistingEntry {
		/// Update the entry's last-read time when this timestamp is present.
		touch_at: Option<Timestamp>,
		/// Continue with this action after the touch, including when the touch
		/// fails under [`StorageWriteFailurePolicy::ContinueWithoutWrite`].
		action: ReadAction<'a, E>,
	},
}

/// Whether a scheduled background refresh that has come due should run.
#[derive(Debug, Eq, PartialEq)]
pub enum DueRefreshDecision {
	/// Start a refresh, or wait for the one already running.
	Run,
	/// The entry is missing or idle. Schedule no new refresh.
	Idle,
}

/// What a read does after any last-read write.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadAction<'a, E> {
	/// Return the stored value within its freshness interval and epoch.
	ServeFresh {
		/// The start time of the fetch that populated the value.
		fetch_started_at: Timestamp,
	},
	/// Return the stored value within the stale window before any failure,
	/// and request an immediate refresh in the background.
	ServeStaleAndRefresh {
		/// The start time of the fetch that populated the value.
		fetch_started_at: Timestamp,
		/// Replace the pending background refresh with this deadline.
		refresh_at: Timestamp,
	},
	/// A previous failure affects the answer. The source's stale-on-error
	/// policy determines whether to serve the stored value. An expired
	/// fallback limit can only produce this action during backoff.
	AnswerFromLastFailure {
		/// Serve the stored value or report its entry's recorded error immediately.
		answer: FailureAnswer<&'a E>,
		/// Request an immediate background refresh only when the stored epoch
		/// matches, the value is within the stale window, and backoff has ended.
		/// Otherwise leave scheduling alone.
		refresh_at: Option<Timestamp>,
	},
	/// Start a refresh, or wait for the one already running, and use its
	/// outcome. This includes a value past the stale-on-error limit once
	/// backoff has ended, even inside the stale window.
	WaitForRefresh,
}

/// What a read can return when it relies on upstream failure handling.
#[derive(Debug, Eq, PartialEq)]
pub enum FailureAnswer<Cause> {
	/// Serve the last good value as stale due to an upstream failure.
	ServeStored {
		/// The start time of the successful fetch, possibly in another epoch.
		fetch_started_at: Timestamp,
	},
	/// No stored value can be served under the source's policy.
	ReportError(Cause),
}

/// What to give a waiting reader when the refresh just failed.
#[derive(Debug, Eq, PartialEq)]
pub enum FailedRefreshAnswer {
	/// Serve the last good value as stale, possibly from a different epoch.
	ServeStored {
		/// The start time of the fetch that produced the stored value.
		fetch_started_at: Timestamp,
	},
	/// Report the error from the refresh that just failed.
	ReportThisRefreshError,
}

/// The error to report after an abort leaves no usable stored value.
#[derive(Debug, Eq, PartialEq)]
pub enum AbortedRefreshError<'a, E> {
	/// Report the stored error from an earlier failed refresh.
	Stored(&'a E),
	/// No failure was recorded. Show exactly `upstream unavailable`.
	///
	/// This is the fallback at the [reference's read return][fallback].
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L130
	UpstreamUnavailable,
}

/// The entry state when a refresh finishes.
#[derive(Debug)]
pub enum RefreshEntry<'a, E> {
	/// Use the latest entry, including any last-read update during the fetch.
	Present(&'a Entry<E>),
	/// The entry is missing without an intentional purge. A successful fetch
	/// can recreate it. The [reference throws][missing] on a missing entry.
	///
	/// [missing]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L242
	Missing {
		/// The last known read time, including the original read if its insert
		/// failed. Preserve this time when recreating the entry.
		last_read: Timestamp,
	},
	/// The source was intentionally purged during this fetch. Do not recreate
	/// its entry. Give waiting readers the outcome even though the
	/// [reference throws][purged] on the missing entry.
	///
	/// [purged]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L242
	Purged,
}

/// What to write and what to give readers after a refresh.
#[derive(Debug, Eq, PartialEq)]
pub enum RefreshDecision<'a, E> {
	/// Write the entry from [`entry_after_success`] and mark the new value fresh
	/// for waiting readers. After the write, use [`decide_next_background_refresh`].
	StoreValue {
		/// Last-read time to preserve when replacing or recreating the entry.
		last_read: Timestamp,
		/// Metadata for the loaded value to store and serve.
		stored: CachedValueMetadata,
	},
	/// A refresh succeeded after a purge. Give waiting readers the new value
	/// marked fresh and leave storage and scheduling alone.
	ReturnFreshWithoutEntry {
		/// The start time of the successful fetch.
		fetch_started_at: Timestamp,
	},
	/// Write the entry from [`entry_after_failure`]. After the write, use
	/// [`decide_next_background_refresh`].
	RecordFailure {
		/// The computed update to persist with the current error.
		update: FailureUpdate,
		/// Serve a stored value or report the current refresh's error.
		answer: FailedRefreshAnswer,
	},
	/// A refresh failed without an entry to update. Report the current
	/// refresh's error and leave storage and scheduling alone.
	ReportThisRefreshErrorWithoutEntry,
	/// A refresh aborted. Leave storage and scheduling alone.
	Aborted {
		/// Serve a stored value, report a stored error, or report unavailable
		/// when no failure was recorded.
		answer: FailureAnswer<AbortedRefreshError<'a, E>>,
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
/// Use the supplied last-read time and value metadata and clear any failure.
/// After writing the entry with the value,
/// [compute the next background refresh](decide_next_background_refresh).
#[must_use]
pub fn entry_after_success<E>(last_read: Timestamp, stored: CachedValueMetadata) -> Entry<E> {
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
	Idle,
}

/// Decide how to answer a read for a registered source.
///
/// Insert a cold miss at `now` and wait for a refresh. For an existing entry,
/// touch `last_read` only when more than [`TOUCH_EVERY`] has passed. Use the
/// value's freshness override when present, including zero. With no current
/// epoch every value matches. Otherwise the stored epoch must be present and
/// equal to the current one (`docs/reference/cache-semantics.md` §7).
///
/// In order:
///
/// 1. Serve fresh if the epoch matches and `age < fresh_for`, regardless of
///    any recorded failure or active backoff.
/// 2. If the epoch matches and `age < fresh_for + stale_window`, serve stale
///    and request an immediate background refresh when no failure is recorded.
///    With a recorded failure, use an eligible fallback and request an
///    immediate background refresh only if backoff has ended. An ineligible
///    fallback proceeds to the next check.
/// 3. If `backoff_until > now`, serve an eligible last good value or report
///    the stored error, without scheduling. Otherwise wait for a refresh.
///
/// The freshness, stale window, and backoff cutoffs are strict. The
/// stale-on-error age limit is inclusive.
#[must_use]
pub fn decide_read<'a, E: Error>(
	policy: FreshnessPolicy,
	now: Timestamp,
	entry: Option<&'a Entry<E>>,
	epoch: Option<&Epoch>,
) -> ReadDecision<'a, E> {
	todo!()
}

/// Decide whether a scheduled background refresh that has come due should run.
///
/// Run for any present entry that is not idle under [`IDLE_AFTER`], regardless
/// of freshness, epoch, or backoff (`docs/reference/cache-semantics.md` §5.2).
/// A missing or idle entry leaves the resource idle.
#[must_use]
pub fn decide_due_refresh<E: Error>(
	now: Timestamp,
	entry: Option<&Entry<E>>,
) -> DueRefreshDecision {
	todo!()
}

/// Decide what to write and how to answer waiting readers when a refresh ends.
///
/// Use `finished_at` to check the age of a possible fallback on failure or
/// abort. On success mark the newly loaded value fresh for waiting readers
/// and write it unless the source was purged. On failure write the next
/// failure count and backoff deadline only if an entry is present, then
/// serve an eligible fallback or report the current refresh's error. On abort
/// leave storage unchanged and use an eligible fallback, a stored error, or
/// the unavailable message in that order.
///
/// Measure backoff from the fetch start. Use the greater of [`MIN_BACKOFF`]
/// doubled for each additional consecutive failure up to [`MAX_BACKOFF`],
/// and the supplied `Retry-After` delay capped at [`MAX_RETRY_AFTER`].
/// See `docs/reference/cache-semantics.md` §6.3 and the [reference calculation].
///
/// [reference calculation]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L219-L232
#[must_use]
pub fn decide_refresh_finished<E: Error>(
	policy: FreshnessPolicy,
	finished_at: Timestamp,
	entry: RefreshEntry<'_, E>,
	outcome: RefreshOutcome,
) -> RefreshDecision<'_, E> {
	todo!()
}

/// Decide when to next refresh in the background after writing a value or failure.
///
/// Pass the entry as written and a fresh timestamp sampled after the write.
/// Cancel the pending refresh if the entry is idle under [`IDLE_AFTER`].
/// Otherwise, schedule at the end of backoff when `backoff_until > now`,
/// or at the value's fetch start plus its effective freshness interval. If there is no stored
/// value and backoff has elapsed, schedule one default freshness interval
/// after the supplied time.
#[must_use]
pub fn decide_next_background_refresh<E: Error>(
	policy: FreshnessPolicy,
	now: Timestamp,
	entry: &Entry<E>,
) -> NextBackgroundRefresh {
	todo!()
}
