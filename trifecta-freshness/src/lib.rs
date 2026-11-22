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
	/// Add a duration, clamping at the implementation's supported ceiling.
	/// The result is never earlier than `self`.
	#[must_use]
	fn saturating_add(self, duration: Duration) -> Self;

	/// Compute `self − other` at the type's precision when nonnegative. Return
	/// [`SignedElapsed::Before`] exactly when `self < other`.
	fn elapsed_since(self, other: Self) -> SignedElapsed;
}

/// An elapsed time that preserves the order of the two timestamps.
#[derive(Debug, Eq, PartialEq)]
pub enum SignedElapsed {
	/// The first timestamp comes before the second.
	Before,
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

/// Saturate additions at 9999-12-31 23:59:59 UTC (253402300799 seconds after
/// the Unix epoch), a fixed ceiling because Rust exposes no portable maximum
/// for `SystemTime`. A time already above the ceiling stays unchanged.
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
	/// How to serve a matching cached value within the stale window when no
	/// failure is recorded.
	pub on_stale: StaleRead,
	/// How to serve an eligible matching cached value within the stale window
	/// after a failure, once backoff has ended.
	pub on_stale_after_failure: StaleRead,
	/// Age limit for serving a cached value after a fetch failure.
	pub serve_cached_after_failure: CachedAgeLimit,
	/// Function that labels a timestamp. Reads use the label for the read time
	/// and successful fetches cache the label for the fetch start. Without a
	/// function, any cached value matches. With one, a missing label cannot match.
	///
	/// The reference uses this only for [`BonApp`'s campus day][campus-day].
	///
	/// [campus-day]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/sources/bonapp.ts#L26
	pub period_of: Option<fn(T) -> ValidityPeriod>,
}

/// How a read uses a matching cached value within the stale window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StaleRead {
	/// Serve the cached value now and enqueue a refresh immediately.
	/// This is the [reference behavior][reference].
	///
	/// [reference]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L116
	#[default]
	EnqueueImmediateRefresh,
	/// Serve the cached value now and refresh at the read time plus this
	/// duration, using saturating addition.
	RefreshAfter(Duration),
	/// Fetch inline and serve the result, using the cached value as fallback
	/// if the fetch fails and the value is still eligible.
	FetchLive,
}

/// How old a cached value may be after a failed or aborted fetch.
///
/// A cached value is eligible if its age at the decision time is within this
/// limit, regardless of its validity period.
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
	/// Record the read time in a new entry with no cached value and no failure.
	#[must_use]
	pub fn new_for_read(t: T) -> Self {
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
	/// The fetch could not start because reading the saved upstream responses
	/// for conditional requests failed. Entry reads follow
	/// [`StorageReadFailurePolicy`]. Failed writes follow
	/// [`StorageWriteFailurePolicy`].
	Aborted,
}

/// What to do after reporting a failed read of the cache entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageReadFailurePolicy {
	/// Return the storage error, as the reference does for an entry read.
	Propagate,
	/// Fetch directly from the source. Then call
	/// [`FreshnessPolicy::on_fetch_finished`] with [`EntryAtFetchEnd::Missing`]
	/// using the read time. Apply the resulting fields as for any fetch,
	/// inserting if the entry is absent. A failed write follows
	/// [`StorageWriteFailurePolicy`].
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
		/// The time to pass to [`Entry::new_for_read`].
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
	/// Serve the cached value as stale and enqueue a refresh at the read
	/// time. Keep any refresh already pending at an earlier time.
	ServeStaleAndEnqueueImmediateRefresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Serve the cached value as stale and request a refresh for later.
	/// Keep any refresh already pending at an earlier time.
	ServeStaleAndRefreshAt {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
		/// When the requested refresh should run.
		refresh_at: T,
	},
	/// Start a fetch or join the one already running. Keep the cached value
	/// already read without copying it. Pass its metadata as the fallback
	/// to [`FreshnessPolicy::on_fetch_finished`] so it remains available on
	/// failure even if the entry is evicted. A purge invalidates the fallback.
	FetchWithStaleFallback {
		/// When the fetch that produced the fallback began.
		fallback_fetch_started_at: T,
	},
	/// Serve the cached value after a failed fetch.
	ServeCachedAfterFailure {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
		/// Whether to request a refresh or report that backoff is active.
		follow_up: RefreshAfterFailure<T>,
	},
	/// Report the cached fetch error immediately during backoff.
	ReportCachedError(&'a E),
	/// Start a fetch or join the one already running. No usable fallback
	/// is cached, so a failed fetch returns its error.
	FetchWithoutFallback,
}

/// What follows serving the cached value after a failure.
#[derive(Debug, Eq, PartialEq)]
pub enum RefreshAfterFailure<T> {
	/// Enqueue a refresh at the read time. Keep any refresh already pending
	/// at an earlier time.
	EnqueueImmediateRefresh,
	/// Request a refresh at this time without waiting. Keep any refresh
	/// already pending at an earlier time.
	RefreshAt(T),
	/// Backoff holds off a fetch until this time. Keep the existing schedule.
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
	/// The entry is absent or unknown after a failed read. Insert if absent,
	/// or apply a partial update if present. A missing entry differs from the
	/// [reference, which throws][missing].
	///
	/// [missing]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L242
	Missing {
		/// Last known read time, or the current read time if the entry is unknown.
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
	/// Cache the new value and clear the failure record. Update only those
	/// fields, preserving any concurrent change to the entry's last-read time.
	/// Serve the fresh value and use the written entry to decide the next refresh.
	StoreValue {
		/// Metadata that replaces the previous cached value's metadata.
		cached: CachedValueMetadata<T>,
	},
	/// Recreate a missing entry after a successful fetch. Write the value and
	/// metadata with no failure record. If a read created the entry in the
	/// meantime, apply the same partial update as [`Self::StoreValue`].
	/// Decide the next refresh from the written entry.
	StoreValueInNewEntry {
		/// Last known read time for insertion only.
		insert_at: T,
		/// Metadata for the new cached value.
		cached: CachedValueMetadata<T>,
	},
	/// Serve the fresh value without caching it because a purge invalidated
	/// this fetch. Leave scheduling alone.
	ServeWithoutCaching {
		/// When the successful fetch began.
		fetch_started_at: T,
	},
	/// Change only the failure record. Use the written entry to decide the
	/// next refresh. The caller retains ownership of the error.
	RecordFailure {
		/// The failure record to set, borrowing the completed fetch's error.
		failure: Failure<T, &'fetch E>,
		/// Calculated delay before clamping the deadline, for the caller's
		/// backoff event.
		backoff_delay: Duration,
		/// Serve the cached value or report this fetch's error.
		answer: FailureAnswer<T, ThisFetchError>,
	},
	/// Set the failure record when the entry is absent or unknown. If
	/// absent, insert with the supplied read time. If present, change only
	/// the failure record. Decide the next refresh from the written entry.
	RecordFailureWithInsertIfMissing {
		/// Read time to use only if inserting.
		insert_at: T,
		/// The failure record to set.
		failure: Failure<T, &'fetch E>,
		/// Calculated delay before clamping the deadline, for the caller's
		/// backoff event.
		backoff_delay: Duration,
		/// Serve an eligible fallback or report this fetch's error.
		answer: FailureAnswer<T, ThisFetchError>,
	},
	/// Report this fetch's error and leave caching and scheduling alone
	/// because a purge invalidated the fetch.
	ReportErrorWithoutCaching,
	/// Answer readers after an aborted fetch without updating storage or scheduling.
	Aborted {
		/// Serve an eligible cached value, report the cached error, or report unavailable.
		answer: FailureAnswer<T, AbortedFetchReport<'entry, E>>,
	},
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
	Cancel,
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
	/// `age < fresh_for + stale_window` (using saturating addition), apply
	/// [`Self::on_stale`] before any recorded failure, regardless of the
	/// fallback age limit. After a recorded failure, apply the age limit.
	/// Serve an eligible cached value with its backoff deadline while backoff
	/// is active. Once it ends, apply [`Self::on_stale_after_failure`] to an
	/// eligible value in the window. Either setting can request a refresh
	/// or fetch inline. A zero delay requests immediate enqueueing.
	///
	/// Outside the window or on a period mismatch, active backoff
	/// (`backoff_until > now`) still allows serving an eligible cached value.
	/// During backoff report the cached error if there is no eligible value.
	/// Otherwise fetch, preserving an eligible cached value for fallback when
	/// available. Reporting an error never requests a refresh.
	///
	/// A read's refresh request always keeps the earlier of the requested
	/// deadline and any pending deadline. Repeated reads cannot defer it.
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
	/// value, and replace the cached metadata and clear the failure record
	/// unless a purge invalidated this fetch. Apply those changes without
	/// writing `last_read`. A successful fetch for a missing entry uses the
	/// last known read time only if it inserts a new entry. A failed fetch
	/// updates the failure record and may likewise insert a missing entry.
	///
	/// Pass the metadata of the cached value retained by the reader as
	/// `fallback`. The entry at completion determines writes separately.
	/// If the fetch fails, evaluate fallback age at `finished_at`; without an
	/// eligible fallback, report the fetch error. Eviction does not remove
	/// the retained fallback. A purge suppresses writes and invalidates the
	/// fallback even if the entry was recreated. On abort, apply the age
	/// limit too. Serve an eligible cached value, otherwise report the cached
	/// error or exactly `upstream unavailable`, in that order.
	///
	/// Backoff starts at fetch start. The exponential part is
	/// `min(MIN_BACKOFF × 2^(n−1), MAX_BACKOFF)` for the new consecutive
	/// failure count `n`. It is 1,920 seconds at failure 7 and
	/// [`MAX_BACKOFF`] from failure 8, computed without overflow.
	/// Take the greater of that and the supplied `Retry-After` capped at
	/// [`MAX_RETRY_AFTER`]. Clamp the count at `u32::MAX` and saturate
	/// deadline addition.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L223).
	#[must_use]
	pub fn on_fetch_finished<'entry, 'fetch, E: Error>(
		&self,
		finished_at: T,
		entry: EntryAtFetchEnd<'entry, T, E>,
		fallback: Option<&CachedValueMetadata<T>>,
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
