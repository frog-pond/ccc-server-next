//! Pure freshness rules for one cache entry at one time.
//!
//! The cached pipeline in `trifecta` (#441) reads an entry, asks
//! [`FreshnessPolicy`] what it means at the current time, and does the storage
//! work itself. This crate decides how to answer a read, when to record the read
//! time, what to record about a new value or a failed fetch, whether a kept value
//! may still be served after a failure, when a refresh is next worth doing, and
//! whether a refresher may run now. Readers that share a fetch, what to write when
//! a fetch ends, and cache failures belong to the pipeline (#441). Refresh
//! requests and the refreshers that handle them belong to background refresh
//! (#446).
//!
//! The rules follow `docs/reference/cache-semantics.md` §§4–9. The crate reads no
//! clock and does no I/O: every time is an argument.
//!
//! The default `std` feature implements [`Timestamp`] for
//! [`std::time::SystemTime`]. Without it the crate needs only `core`, and the
//! caller supplies its own [`Timestamp`] type.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
// Temporary while the draft signatures are under review.
#![allow(
	unused_variables,
	clippy::missing_panics_doc,
	clippy::needless_pass_by_value
)]

use core::{num::NonZeroU32, time::Duration};

/// A read rewrites the last-read time only after more than this interval.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L20).
pub const TOUCH_EVERY: Duration = Duration::from_secs(60 * 10);

/// An entry is idle only when more than this interval has passed since its
/// recorded last read. An idle entry gets no refresh hint and may not be
/// refreshed.
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
///
/// Implemented for [`std::time::SystemTime`] with the default `std` feature, and
/// for `jiff::Timestamp` with the `jiff` feature. A caller without `std`
/// implements it for its own type. A Cloudflare Worker can use `SystemTime`
/// built as `UNIX_EPOCH + Duration::from_millis(Date.now())`: arithmetic works
/// on `wasm32-unknown-unknown`, but `SystemTime::now()` panics there, so nothing
/// handed to the policy may call it.
///
/// Implementations must be pure: no clock, no I/O, and the same result for the
/// same arguments.
pub trait Timestamp: Copy + Ord {
	/// Add a duration, clamping at the implementation's supported ceiling.
	/// The result is never earlier than `self`.
	#[must_use]
	fn saturating_add_duration(self, duration: Duration) -> Self;

	/// The time elapsed from `earlier` to `self` at the type's precision, or
	/// `None` exactly when `self < earlier`. Ages are compared signed, as the
	/// reference does, so a value whose fetch started after `now` has no age and
	/// counts as younger than any limit.
	#[must_use]
	fn checked_duration_since(self, earlier: Self) -> Option<Duration>;
}

/// Saturate additions at 9999-12-31 23:59:59 UTC (253402300799 seconds after
/// the Unix epoch), a fixed ceiling because Rust exposes no portable maximum
/// for `SystemTime`. A time already above the ceiling stays unchanged.
#[cfg(feature = "std")]
impl Timestamp for std::time::SystemTime {
	fn saturating_add_duration(self, duration: Duration) -> Self {
		todo!()
	}

	fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
		todo!()
	}
}

/// Saturate additions at `jiff::Timestamp::MAX`, through jiff's own
/// `saturating_add`.
#[cfg(feature = "jiff")]
impl Timestamp for jiff::Timestamp {
	fn saturating_add_duration(self, duration: Duration) -> Self {
		todo!()
	}

	fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
		todo!()
	}
}

/// The freshness settings for one source, as the reference's
/// [`defineSource`][define] declares them.
///
/// [define]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L1-L19
#[derive(Clone, Copy, Debug)]
pub struct FreshnessPolicy<T> {
	/// Default freshness interval measured from fetch start. A value may carry
	/// its own interval in [`CachedValueMetadata::fresh_for`].
	pub fresh_for: Duration,
	/// Additional time after freshness when a read is answered with the cached
	/// value and asks for a refresh, while the value's period is current. This is
	/// the reference's [`staleIfError`][window], described in
	/// `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_window: Duration,
	/// How long before the value goes stale a read may ask for a refresh, and a
	/// refresher may run, so that a value read often never goes stale. Zero is
	/// the reference's behaviour. Anything else is an opt-in difference from the
	/// reference, agreed in #439. A value at or above the effective freshness
	/// interval allows a refresh at any time.
	pub refresh_ahead: Duration,
	/// Age limit, measured from fetch start, for serving a cached value after a
	/// fetch fails. `None` serves it at any age, as the
	/// [reference does][fallback]. A limit is an opt-in from #439. Past it, a read
	/// during backoff reports the cached error, and a read after backoff waits for
	/// a fetch even within the stale window.
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L129
	pub serve_cached_after_failure: Option<Duration>,
	/// The end of the validity period that contains a fetch's start time. With a
	/// function, a cached value counts as fresh or stale only before its stored
	/// [`CachedValueMetadata::period_ends_at`], and a value stored without one
	/// never does. Without a function, any stored end is ignored. The function
	/// must be pure.
	///
	/// The reference labels each value with [`BonApp`'s campus day][campus-day]
	/// and compares labels. An end time gives the same answers except when the
	/// clock is set back past a period boundary, which then counts as the same
	/// period. This difference is agreed in #439.
	///
	/// [campus-day]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/sources/bonapp.ts#L26
	pub period_end: Option<fn(T) -> T>,
}

/// The record a cache backend keeps for one resource beside its optional value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry<T, E> {
	/// Time recorded when the entry is created, and on a read more than
	/// [`TOUCH_EVERY`] later (see [`FreshnessPolicy::record_read`]).
	pub last_read: T,
	/// Metadata present exactly when the cache holds a value.
	pub cached: Option<CachedValueMetadata<T>>,
	/// The failure streak since the last successful fetch, if any.
	pub failure: Option<Failure<T, E>>,
}

/// What the entry records about its cached value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedValueMetadata<T> {
	/// When the successful fetch began.
	pub fetch_started_at: T,
	/// The end of the validity period that contains `fetch_started_at`, from
	/// [`FreshnessPolicy::period_end`]. Absent when the source has no periods.
	/// The value is still served as a fallback after this time.
	pub period_ends_at: Option<T>,
	/// A source-computed freshness interval for this value, including zero when
	/// it expires immediately: the reference's
	/// [`ttlFor`](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L208).
	/// Absence uses [`FreshnessPolicy::fresh_for`].
	pub fresh_for: Option<Duration>,
}

/// A recorded fetch failure. Backoff can end while the record persists; it is
/// cleared only by a successful fetch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure<T, E> {
	/// Consecutive failed fetches, including the latest, clamped at `u32::MAX`.
	pub consecutive_failures: NonZeroU32,
	/// Fetches are held off while this time is strictly later than now.
	pub backoff_until: T,
	/// The most recent fetch error.
	pub last_error: E,
}

/// How to answer a read, given the entry at the read time.
///
/// The `Request…` variants ask the pipeline's refresher (#446) for a refresh.
/// The refresher decides whether the reader waits for it. An inline refresher
/// makes the reader wait, with the cached value as its fallback. One that
/// sidetracks the request, such as a Durable Object alarm, lets the reader be
/// answered with the cached value at once, as the reference does.
#[derive(Debug, Eq, PartialEq)]
pub enum ReadAction<'a, T, E> {
	/// Serve the cached value as fresh. Fresh values are served even during
	/// backoff.
	ServeFresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Serve the cached value as fresh, and ask for a refresh because the value
	/// is within [`FreshnessPolicy::refresh_ahead`] of going stale and no backoff
	/// is active. The reader never waits for this refresh.
	ServeFreshAndRequestRefresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// The value is in its stale window and no failure is recorded. Ask for a
	/// refresh. If the reader is not made to wait, serve the value as `stale`.
	RequestRefresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// The value is in its stale window, a failure is recorded, backoff has
	/// ended, and the value is within the age limit. Ask for a refresh. If the
	/// reader is not made to wait, serve the value as `stale-error`.
	RequestRefreshAfterFailure {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Backoff is active and the value is within the age limit. Serve it as
	/// `stale-error`, whatever its period, without asking for a refresh.
	ServeCachedAfterFailure {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Backoff is active and no cached value is within the age limit. Report the
	/// recorded error at once.
	ReportCachedError(&'a E),
	/// Nothing can be answered now: start a fetch or join the one in flight. The
	/// cached value is within the age limit, so give it to the fetch as the
	/// fallback for a failure.
	FetchWithStaleFallback {
		/// When the fetch that produced the fallback began.
		fallback_fetch_started_at: T,
	},
	/// Nothing can be answered now and there is no usable fallback: start a fetch
	/// or join the one in flight. A failed fetch reports its error. A read with no
	/// entry gets this action.
	FetchWithoutFallback,
}

impl<T: Timestamp> FreshnessPolicy<T> {
	/// Decide how to answer a read.
	///
	/// A value is current when the policy has no [`Self::period_end`], or the
	/// value's stored period end is later than `now`. Its age is
	/// `now − fetch_started_at`, compared signed: a fetch that started after `now`
	/// is younger than any interval. Its freshness interval is its own
	/// [`CachedValueMetadata::fresh_for`] if present, including zero, and
	/// otherwise [`Self::fresh_for`]. Backoff is active while
	/// `backoff_until > now`.
	///
	/// In order:
	/// 1. A current value younger than its freshness interval is served fresh,
	///    even during backoff. Within [`Self::refresh_ahead`] of that interval and
	///    outside backoff, the read also asks for a refresh.
	/// 2. A current value younger than its freshness interval plus
	///    [`Self::stale_window`] (saturating) asks for a refresh when no failure is
	///    recorded, regardless of the age limit. With a failure recorded, it is
	///    served after the failure during backoff, or asks for a refresh after
	///    backoff, if it is within the age limit.
	/// 3. Otherwise, during backoff, a value within the age limit is served after
	///    the failure, and without one the recorded error is reported.
	/// 4. Otherwise the read fetches, keeping a value within the age limit as the
	///    fallback.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L106-L130).
	#[must_use]
	pub fn on_read<'a, E>(&self, now: T, entry: Option<&'a Entry<T, E>>) -> ReadAction<'a, T, E> {
		todo!()
	}

	/// The time to record as the entry's last read, if any: `now` when there is
	/// no entry, or when more than [`TOUCH_EVERY`] has passed since the recorded
	/// last read. Otherwise `None`, including when the recorded time is after
	/// `now`.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L92-L104).
	#[must_use]
	pub fn record_read<E>(&self, now: T, entry: Option<&Entry<T, E>>) -> Option<T> {
		todo!()
	}

	/// What to record about a value from a successful fetch that began at
	/// `started_at`. `fresh_for` is the value's own freshness interval, if its
	/// source computes one. The period end is computed from `started_at`, so a
	/// fetch that crosses a period boundary belongs to the period it started in.
	/// A success also clears the failure record.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L202-L209).
	#[must_use]
	pub fn cached_after_success(
		&self,
		started_at: T,
		fresh_for: Option<Duration>,
	) -> CachedValueMetadata<T> {
		todo!()
	}

	/// The failure record after a fetch that began at `started_at` failed with
	/// `error`, and the backoff delay for the `ccc.source.backoff_ms` event.
	///
	/// The count is one more than `previous`, clamped at `u32::MAX`. The
	/// exponential part is `min(MIN_BACKOFF × 2^(n−1), MAX_BACKOFF)` for the new
	/// count `n`: 1,920 seconds at failure 7 and [`MAX_BACKOFF`] from failure 8,
	/// computed without overflow. The delay is the greater of that and
	/// `retry_after` capped at [`MAX_RETRY_AFTER`]. Any `retry_after` is
	/// accepted, zero when the upstream sent none. Backoff ends at `started_at`
	/// plus the delay, saturating. The returned delay is before that saturation.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L219-L232).
	#[must_use]
	pub fn failure_after<E, F>(
		&self,
		started_at: T,
		previous: Option<&Failure<T, E>>,
		error: F,
		retry_after: Duration,
	) -> (Failure<T, F>, Duration) {
		todo!()
	}

	/// Whether a kept value whose fetch began at `fallback_fetch_started_at` may
	/// be served after a failed fetch, at `now`: always without an age limit,
	/// otherwise when its signed age is at most the limit. Its period does not
	/// matter.
	#[must_use]
	pub fn may_serve_fallback(&self, now: T, fallback_fetch_started_at: T) -> bool {
		todo!()
	}

	/// A hint for a refresher that schedules: the next time a refresh would do
	/// any good. A refresher may ignore it. An inline refresher never uses it.
	///
	/// `None` when the entry is idle under [`IDLE_AFTER`]: stop scheduling it.
	/// During backoff, when backoff ends: the earliest a retry is allowed.
	/// Otherwise, for a cached value, its fetch start plus its freshness interval
	/// less [`Self::refresh_ahead`], and without one, `now` plus
	/// [`Self::fresh_for`]. A time at or before `now` means a refresh is due.
	/// Additions saturate. Period ends are not scheduled, as in the reference.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L239-L249).
	#[must_use]
	pub fn next_refresh_hint<E>(&self, now: T, entry: &Entry<T, E>) -> Option<T> {
		todo!()
	}

	/// Whether a refresher may run a refresh now. False when there is no entry,
	/// when it is idle under [`IDLE_AFTER`], during backoff, or while its current
	/// value is younger than its freshness interval less
	/// [`Self::refresh_ahead`]. True otherwise.
	///
	/// The reference's [`alarm()`][alarm] checks only idleness, but it is never
	/// set for a time when freshness or backoff would stop it. This check, agreed
	/// in #439, stops a refresher that runs early, such as a queue, from fetching
	/// too soon.
	///
	/// [alarm]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L251-L260
	#[must_use]
	pub fn may_refresh<E>(&self, now: T, entry: Option<&Entry<T, E>>) -> bool {
		todo!()
	}
}
