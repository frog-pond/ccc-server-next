//! Pure cache policy for reads, refreshes, and storage failures.
//!
//! The cached pipeline in `trifecta` supplies metadata, times, and fetch
//! outcomes. This crate computes decisions and entry updates. It calls no
//! external services or pipeline code. Its rules cover reads, refreshes, backoff,
//! validity periods, value-dependent lifetimes, and idle resources
//! in `docs/reference/cache-semantics.md` §§4–9.

#![deny(missing_docs)]
// Temporary while the draft signatures are under review.
#![allow(
	unused_variables,
	clippy::missing_panics_doc,
	clippy::needless_pass_by_value
)]

use std::{
	error::Error,
	num::NonZeroU32,
	time::{Duration, SystemTime},
};

/// A read rewrites the last-read time only after more than this interval.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L20).
pub const TOUCH_EVERY: Duration = Duration::from_secs(60 * 10);

/// An entry is idle only when more than this interval has passed since its
/// recorded last read. Both refresh decisions use this rule.
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

/// A point in time supplied by the caller.
pub trait Timestamp: Copy + Ord {
	/// Add a duration, clamping at the latest representable time.
	#[must_use]
	fn saturating_add(self, duration: Duration) -> Self;

	/// Return the signed difference from `other`, at the type's precision.
	fn elapsed_since(self, other: Self) -> SignedElapsed;
}

/// An elapsed time that preserves the order of the two timestamps.
#[derive(Debug, Eq, PartialEq)]
pub enum SignedElapsed {
	/// A negative elapsed time, carrying the magnitude.
	Before(Duration),
	/// Zero or a positive elapsed time.
	AtOrAfter(Duration),
}

/// Whole milliseconds since the Unix epoch, from `i64::MIN` to `i64::MAX`.
///
/// Deadline addition rounds partial milliseconds up and clamps at `i64::MAX`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnixMillis(i64);

impl UnixMillis {
	/// Construct a timestamp from any signed Unix millisecond count.
	#[must_use]
	pub const fn from_unix_millis(millis: i64) -> Self {
		todo!()
	}

	/// Return the signed Unix millisecond count.
	#[must_use]
	pub const fn as_unix_millis(self) -> i64 {
		todo!()
	}
}

impl Timestamp for UnixMillis {
	fn saturating_add(self, duration: Duration) -> Self {
		todo!()
	}

	fn elapsed_since(self, other: Self) -> SignedElapsed {
		todo!()
	}
}

impl Timestamp for SystemTime {
	fn saturating_add(self, duration: Duration) -> Self {
		todo!()
	}

	fn elapsed_since(self, other: Self) -> SignedElapsed {
		todo!()
	}
}

#[cfg(feature = "jiff")]
impl Timestamp for jiff::Timestamp {
	fn saturating_add(self, duration: Duration) -> Self {
		todo!()
	}

	fn elapsed_since(self, other: Self) -> SignedElapsed {
		todo!()
	}
}

/// A source-defined label for the period a value belongs to. A cached value
/// with a different label can only be served after a fetch failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidityPeriod(pub String);

/// The policy for one source, using caller-supplied timestamps.
#[derive(Clone, Copy, Debug)]
pub struct FreshnessPolicy<T> {
	/// Default freshness interval measured from fetch start.
	pub fresh_for: Duration,
	/// Additional time after freshness when a read can answer immediately and
	/// request a refresh, if the period matches. This is the reference's
	/// [`staleIfError`][window], described in `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_window: Duration,
	/// Age limit for serving a cached value after a fetch failure.
	pub serve_cached_after_failure: CachedAgeLimit,
	/// Function that labels a timestamp. Reads use the label for the read time
	/// and successful fetches cache the label for the fetch start. Without a
	/// function, any cached value matches. With one, a missing label cannot match.
	///
	/// The reference uses this only for `BonApp`'s campus day. The campus-day
	/// function will live in `ccc-freshness`.
	pub period_of: Option<fn(T) -> ValidityPeriod>,
}

/// How old a cached value may be when used after a fetch failure.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CachedAgeLimit {
	/// Serve at any age, as the [reference does][fallback].
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L129
	#[default]
	Unlimited,
	/// Serve up to and including this age measured from fetch start, an opt-in
	/// limit from #439. Beyond it, report the cached error during backoff.
	/// Once backoff ends, wait for a fetch even within the stale window.
	UpTo(Duration),
}

/// The metadata a backend keeps for one resource beside its optional cached value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry<T, E> {
	/// Time recorded at insertion or on a read more than [`TOUCH_EVERY`] later.
	pub last_read: T,
	/// Metadata present exactly when the cache holds a value.
	pub cached: Option<CachedValueMetadata<T>>,
	/// The failure streak since the last successful fetch, if any.
	pub failure: Option<Failure<T, E>>,
}

impl<T, E> Entry<T, E> {
	/// Build an entry for a cold read at `t`, with no cached value or failure.
	#[must_use]
	pub fn new_unread(t: T) -> Self {
		todo!()
	}

	/// Build the entry to write after a failed fetch. Preserve the last-read
	/// time and cached metadata, and replace the failure record.
	#[must_use]
	pub fn record_failure<F>(&self, update: FailureUpdate<T>, error: F) -> Entry<T, F>
	where
		T: Clone,
	{
		todo!()
	}
}

/// Metadata for the cached value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedValueMetadata<T> {
	/// When the successful fetch began.
	pub fetch_started_at: T,
	/// The label at fetch start, absent when the source has no validity periods.
	pub period: Option<ValidityPeriod>,
	/// A source-computed freshness interval for this value, including zero
	/// when it expires immediately. Absence uses the default freshness interval.
	pub fresh_for: Option<Duration>,
}

/// A recorded fetch failure. Backoff can expire while the record persists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure<T, E> {
	/// Consecutive failed fetches, including this one, clamped at `u32::MAX`.
	pub consecutive_failures: NonZeroU32,
	/// The fetch is held off while this timestamp is strictly later than now.
	pub backoff_until: T,
	/// The most recent fetch error.
	pub last_error: E,
}

/// The upstream outcome or the reason the fetch could not start.
#[derive(Debug)]
pub enum FetchOutcome<'a, T, E> {
	/// The caller has a new value to serve or cache.
	Succeeded {
		/// When the fetch began.
		fetch_started_at: T,
		/// The source-computed freshness override for the new value, if any.
		fresh_for: Option<Duration>,
	},
	/// The upstream fetch failed. The caller keeps the error.
	Failed {
		/// When the fetch began. Backoff is measured from this time.
		fetch_started_at: T,
		/// The current fetch's error held by the caller.
		error: &'a E,
		/// The longest `Retry-After` from this fetch, or zero if absent.
		/// Any value is accepted. The decision caps it at [`MAX_RETRY_AFTER`].
		retry_after: Duration,
	},
	/// A required storage read failed before the fetch could start. A failed
	/// write after success follows [`StorageWriteFailurePolicy`].
	Aborted,
}

/// What to do after reporting a storage read failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageReadFailurePolicy {
	/// Return the storage error.
	Propagate,
	/// Fetch directly from the source and return its outcome.
	FetchFromSource,
}

/// What to do after reporting a storage write failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageWriteFailurePolicy {
	/// Return the storage error.
	Propagate,
	/// Continue the read action after a failed insert or touch. After a
	/// failed write at fetch completion, return the answer without scheduling.
	ContinueWithoutWrite,
}

/// What to write and how to answer a read.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadDecision<'a, T, E> {
	/// Insert an empty entry, then start a fetch or wait for the one already
	/// running. After an insert failure, follow the storage write policy.
	InsertAndWaitForFetch {
		/// The time to pass to [`Entry::new_unread`].
		insert_at: T,
	},
	/// Answer using the entry and the requested action.
	UseExistingEntry {
		/// Update the last-read time first when this timestamp is present.
		touch_at: Option<T>,
		/// The action to carry out, including if a failed touch is ignored.
		action: ReadAction<'a, T, E>,
	},
}

/// What to do with a read after any last-read write.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadAction<'a, T, E> {
	/// Serve the cached value as fresh.
	ServeFresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Serve the cached value as stale and request a refresh without waiting.
	ServeStaleAndFetch {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
		/// When the refresh should run.
		fetch_at: T,
	},
	/// Serve the cached value after a failed fetch.
	ServeCachedAfterFailure {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
		/// Whether to request a refresh or report that backoff is active.
		fetch: FetchAfterFailure<T>,
	},
	/// Report the cached fetch error immediately during backoff.
	ReportCachedError(&'a E),
	/// Start a fetch or wait for the one already running. This includes an
	/// over-age cached fallback after backoff ends, even within the stale window.
	WaitForFetch,
}

/// Timing of the next fetch when a cached value is served after a failure.
#[derive(Debug, Eq, PartialEq)]
pub enum FetchAfterFailure<T> {
	/// Request a refresh at this time without waiting.
	FetchAt(T),
	/// A fetch is held off until this time. Keep the existing schedule.
	HeldOffUntil(T),
}

/// What to give a reader after a failed or aborted fetch.
#[derive(Debug, Eq, PartialEq)]
pub enum FailureAnswer<T, Report> {
	/// Serve the cached value, which may belong to a different period.
	ServeCached {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// The error to report when there is no eligible cached value.
	ReportError(Report),
}

/// Instruct the caller to report this fetch's error, which it already holds.
#[derive(Debug, Eq, PartialEq)]
pub struct ThisFetchError;

/// What to report if there is no cached value after an aborted fetch.
#[derive(Debug, Eq, PartialEq)]
pub enum AbortedFetchReport<'a, E> {
	/// Report the cached error.
	CachedError(&'a E),
	/// Report exactly `upstream unavailable`, as in the
	/// [reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L130).
	UpstreamUnavailable,
}

/// The entry state when the fetch ended.
#[derive(Debug)]
pub enum EntryAtFetchEnd<'a, T, E> {
	/// The latest entry, including any read made during the fetch.
	Present(&'a Entry<T, E>),
	/// The entry is missing without a purge. A successful fetch recreates it.
	/// This differs from the [reference, which throws][missing].
	///
	/// [missing]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L242
	Missing {
		/// Last known read time, including the original read if its insert failed.
		last_read: T,
	},
	/// This fetch was invalidated by a purge, even if a read recreated the entry.
	/// The caller must preserve this fact through the write. Waiting readers get
	/// the outcome and the fetch writes nothing, unlike the [reference][missing].
	///
	/// [missing]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L242
	Purged,
}

/// What to write and how to answer waiting readers when a fetch ends.
#[derive(Debug, Eq, PartialEq)]
pub enum FetchDecision<'entry, 'fetch, T, E> {
	/// Cache the new value, serve it as fresh, and use the written entry to
	/// decide the next refresh.
	StoreValue {
		/// The entry with the new metadata and no failure record.
		entry: Entry<T, E>,
	},
	/// Serve the fresh value without caching it because a purge invalidated
	/// this fetch.
	ServeWithoutCaching {
		/// When the successful fetch began.
		fetch_started_at: T,
	},
	/// Write the failure record, then use the written entry to decide the
	/// next refresh. The caller retains ownership of the error.
	RecordFailure {
		/// The entry to write, borrowing the error from the completed fetch.
		entry: Entry<T, &'fetch E>,
		/// Serve the cached value or report this fetch's error.
		answer: FailureAnswer<T, ThisFetchError>,
	},
	/// Report this fetch's error and leave caching and scheduling alone because
	/// the entry is missing or a purge invalidated the fetch.
	ReportErrorWithoutCaching,
	/// Answer readers after an aborted fetch without updating storage or scheduling.
	Aborted {
		/// Serve the cached value, report the cached error, or report unavailable.
		answer: FailureAnswer<T, AbortedFetchReport<'entry, E>>,
	},
}

/// A failure update to record with this fetch's error.
#[derive(Debug, Eq, PartialEq)]
pub struct FailureUpdate<T> {
	/// The new failure count, clamped at `u32::MAX`.
	pub consecutive_failures: NonZeroU32,
	/// The time at which backoff ends.
	pub backoff_until: T,
}

/// Whether to run a refresh at its scheduled time.
#[derive(Debug, Eq, PartialEq)]
pub enum Refresh {
	/// Start a fetch or wait for the one already running.
	Run,
	/// Leave a missing or idle entry alone.
	Skip,
}

/// When to refresh after writing a fetch outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NextRefresh<T> {
	/// Replace the pending refresh. A time already due requests it immediately.
	At(T),
	/// Cancel the pending refresh because the entry is idle.
	None,
}

impl<T: Timestamp> FreshnessPolicy<T> {
	/// Decide how to answer a read for a registered source.
	///
	/// A cold read inserts at `now` and waits for a fetch. Otherwise touch
	/// only after more than [`TOUCH_EVERY`]. Use the cached freshness override
	/// when present, including zero. Compare age signed. A fetch started one
	/// millisecond after `now` has negative age and can be fresh even with
	/// zero freshness.
	///
	/// First serve fresh when the period matches and `age < fresh_for`, even
	/// during backoff. Next, if the period matches and
	/// `age < fresh_for + stale_window` (with saturating duration addition),
	/// serve stale and request a refresh before any failure. After a failure,
	/// serve an eligible cached value and request a refresh only if backoff
	/// has ended. If the cached value is ineligible, proceed to the next check.
	/// Then, if `backoff_until > now`, serve an eligible cached value or report
	/// the cached error. Otherwise wait for a fetch. An eligibility limit for
	/// serving a cached value after failure is inclusive.
	#[must_use]
	pub fn on_read<'a, E: Error>(
		&self,
		now: T,
		entry: Option<&'a Entry<T, E>>,
	) -> ReadDecision<'a, T, E> {
		todo!()
	}

	/// Decide whether a refresh should run. Run for any present entry that is
	/// not idle under [`IDLE_AFTER`], regardless of freshness, period, or backoff.
	#[must_use]
	pub fn on_refresh<E: Error>(&self, now: T, entry: Option<&Entry<T, E>>) -> Refresh {
		todo!()
	}

	/// Decide what to cache and what to tell waiting readers.
	///
	/// On success, compute the validity period at fetch start, serve the new
	/// value, and write an entry unless a purge invalidated this fetch. A
	/// successful fetch for a missing entry preserves the last known read time.
	/// Use `finished_at` to evaluate cached age after failure. Record a failure
	/// only on a present entry. A purge suppresses writes even if the entry was
	/// recreated. On abort, serve the cached value if present, otherwise report
	/// the cached error or exactly `upstream unavailable`, in that order.
	///
	/// Backoff starts at fetch start. Take the greater of the exponential delay
	/// and the supplied `Retry-After` capped at [`MAX_RETRY_AFTER`]. Start at
	/// [`MIN_BACKOFF`] and double per additional failure, stopping at
	/// [`MAX_BACKOFF`] before a multiplication could exceed it. Clamp the count
	/// at `u32::MAX` and saturate deadline addition.
	#[must_use]
	pub fn on_fetch_finished<'entry, 'fetch, E: Error>(
		&self,
		finished_at: T,
		entry: EntryAtFetchEnd<'entry, T, E>,
		outcome: FetchOutcome<'fetch, T, E>,
	) -> FetchDecision<'entry, 'fetch, T, E> {
		todo!()
	}

	/// Decide the next refresh using the written entry and a fresh timestamp.
	///
	/// Cancel if the entry is idle under [`IDLE_AFTER`]. Otherwise use active
	/// backoff's end, or fetch start plus the effective freshness interval.
	/// Without a cached value or active backoff, use now plus the default
	/// freshness interval. Deadline addition saturates.
	#[must_use]
	pub fn next_refresh<E: Error>(&self, now: T, entry: &Entry<T, E>) -> NextRefresh<T> {
		todo!()
	}
}
