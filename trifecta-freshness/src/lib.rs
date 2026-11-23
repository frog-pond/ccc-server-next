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
//! `std::time::SystemTime`. The optional `jiff` feature implements it for
//! `jiff::Timestamp`. Without either, the crate needs only `core`, and the
//! caller supplies its own [`Timestamp`] type. [`Error`] implements
//! `core::error::Error` either way.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use core::{fmt, num::NonZeroU32, time::Duration};

#[cfg(all(test, feature = "std"))]
mod tests;

/// Why a value's freshness record could not be built.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Error {
	/// The value's validity period ends at or before its fetch started: the name
	/// covers "at" too. A period contains the fetch's start, so its end is
	/// strictly later. This comes from
	/// a faulty [`FreshnessPolicy::period_end`] function, or from a stored record
	/// that was written wrongly or has been damaged.
	PeriodEndsBeforeFetch,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::PeriodEndsBeforeFetch => f.write_str("validity period ends at or before fetch start"),
		}
	}
}

impl core::error::Error for Error {}

/// A read rewrites the last-read time only after more than this interval.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L20).
pub const TOUCH_EVERY: Duration = Duration::from_mins(10);

/// An entry is idle only when more than this interval has passed since its
/// recorded last read. An idle entry gets no refresh hint and may not be
/// refreshed.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L18).
pub const IDLE_AFTER: Duration = Duration::from_hours(48);

/// Base backoff for the first consecutive failure, measured from fetch start.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L15).
pub const MIN_BACKOFF: Duration = Duration::from_secs(30);

/// Upper bound for the exponential part of backoff.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L16).
pub const MAX_BACKOFF: Duration = Duration::from_hours(1);

/// Longest upstream-requested delay accepted from `Retry-After`.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/conditional.ts#L53).
pub const MAX_RETRY_AFTER: Duration = Duration::from_hours(24);

/// A point in time supplied by the caller.
///
/// Implemented for `std::time::SystemTime` with the default `std` feature, and
/// for `jiff::Timestamp` with the `jiff` feature. A caller without `std`
/// implements it for its own type. A Cloudflare Worker can use `SystemTime`
/// built as `UNIX_EPOCH.checked_add(Duration::from_millis(Date.now()))`, which
/// returns `None` rather than panicking on overflow. Arithmetic works on
/// `wasm32-unknown-unknown`, but `SystemTime::now()` panics there, so nothing
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
		let ceiling = std::time::UNIX_EPOCH
			.checked_add(Duration::from_secs(253_402_300_799))
			.unwrap_or(self);
		self
			.checked_add(duration)
			.unwrap_or(ceiling)
			.min(ceiling)
			.max(self)
	}

	fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
		self.duration_since(earlier).ok()
	}
}

/// Saturate additions at `jiff::Timestamp::MAX`, through jiff's own
/// `saturating_add`. That returns a `Result`, but its error case applies only to
/// calendar spans with units larger than hours, never to a `Duration`; the body
/// maps it to `jiff::Timestamp::MAX` without unwrapping.
#[cfg(feature = "jiff")]
impl Timestamp for jiff::Timestamp {
	fn saturating_add_duration(self, duration: Duration) -> Self {
		self.saturating_add(duration).unwrap_or(Self::MAX)
	}

	fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
		Duration::try_from(self.duration_since(earlier)).ok()
	}
}

/// The freshness settings for one source, as the reference's
/// [`defineSource`][define] declares them.
///
/// [define]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L1-L19
#[derive(Clone, Copy, Debug)]
pub struct FreshnessPolicy<T> {
	/// Default freshness interval measured from fetch start. A value may carry
	/// its own interval, read with [`CachedValueMetadata::fresh_for`].
	pub fresh_for: Duration,
	/// Additional time after freshness during which a read asks for a refresh
	/// instead of fetching, while the value's period is current. Whether the
	/// reader then waits for that refresh or is answered with the cached value is
	/// the refresher's choice (see [`ReadAction`]). This is the reference's
	/// [`staleIfError`][window], described in
	/// `docs/reference/cache-semantics.md` §6.6.
	///
	/// [window]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/define-source.ts#L13
	pub stale_window: Duration,
	/// How long before a value goes stale a refresh becomes worth doing, so that
	/// a value read often never goes stale. Zero is the reference's behaviour.
	/// Anything else is an opt-in difference from the reference, agreed in #439.
	///
	/// This sets a value's *refresh point*, the one rule that
	/// [`Self::on_read`], [`Self::next_refresh_hint`] and [`Self::may_refresh`]
	/// share. It is an age, so it is compared with the value's signed age and
	/// never with a computed time that could saturate. When the lead is shorter
	/// than the value's freshness interval, a value reaches its refresh point
	/// once its age is at least the interval less the lead. Otherwise the lead is
	/// ignored, and it reaches it once its age is at least the interval, when it
	/// goes stale. So a value fresh for one minute under a ten-minute lead is
	/// refreshed after its minute, not continuously, and a value fresh for zero
	/// is due at once, as in the reference.
	pub refresh_ahead: Duration,
	/// Age limit, measured from fetch start, for serving a cached value as a
	/// fallback after a failure. `None` serves it at any age, as the
	/// [reference does][fallback]. A limit is an opt-in from #439. It applies
	/// only where the value would be served as a fallback: a fresh value is
	/// served whatever the limit, and so is a stale one while no failure is
	/// recorded. Past it, a read during backoff reports the cached error, and a
	/// read after backoff waits for a fetch even within the stale window.
	///
	/// [fallback]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L129
	pub serve_cached_after_failure: Option<Duration>,
	/// The end of the validity period that contains a fetch's start time, which
	/// must be strictly later than that start. With a function, a cached value
	/// counts as fresh or stale only before its stored
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Entry<T, E> {
	/// Time recorded when the entry is created, and on a read more than
	/// [`TOUCH_EVERY`] later (see [`record_read`]).
	pub last_read: T,
	/// Metadata present exactly when the cache holds a value.
	pub cached: Option<CachedValueMetadata<T>>,
	/// The failure streak since the last successful fetch, if any. The caller
	/// sets it to `None` when it stores a new value.
	pub failure: Option<Failure<T, E>>,
}

/// What the entry records about its cached value.
///
/// Its fields are private because a period must end strictly after the fetch
/// started. [`FreshnessPolicy::cached_after_success`] builds one for a new
/// value, and a backend restoring a stored record uses [`Self::new`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CachedValueMetadata<T> {
	fetch_started_at: T,
	period_ends_at: Option<T>,
	fresh_for: Option<Duration>,
}

impl<T: Timestamp> CachedValueMetadata<T> {
	/// Build the record for a value whose fetch began at `fetch_started_at`.
	///
	/// `period_ends_at` is the end of the validity period containing that start,
	/// or `None` when no period end was stored. `fresh_for` is the value's own
	/// freshness interval, the reference's
	/// [`ttlFor`](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L208),
	/// including zero when the value expires at once; absence uses
	/// [`FreshnessPolicy::fresh_for`].
	///
	/// # Errors
	///
	/// [`Error::PeriodEndsBeforeFetch`] when `period_ends_at` is at or before
	/// `fetch_started_at`. A backend that reads such a record treats the entry
	/// as unreadable (#441).
	pub fn new(
		fetch_started_at: T,
		period_ends_at: Option<T>,
		fresh_for: Option<Duration>,
	) -> Result<Self, Error> {
		if period_ends_at.is_some_and(|end| end <= fetch_started_at) {
			return Err(Error::PeriodEndsBeforeFetch);
		}
		Ok(Self {
			fetch_started_at,
			period_ends_at,
			fresh_for,
		})
	}

	/// When the successful fetch began.
	#[must_use]
	pub fn fetch_started_at(&self) -> T {
		self.fetch_started_at
	}

	/// The end of the validity period that contains the fetch's start, strictly
	/// after it, or `None` when no period end was stored. With a
	/// [period function](FreshnessPolicy::period_end), a value with no stored
	/// end is never fresh or stale. The period has no effect on whether the
	/// value may be served as a fallback, which
	/// [`FreshnessPolicy::may_serve_fallback`] decides by age alone.
	#[must_use]
	pub fn period_ends_at(&self) -> Option<T> {
		self.period_ends_at
	}

	/// The value's own freshness interval, if its source computed one.
	#[must_use]
	pub fn fresh_for(&self) -> Option<Duration> {
		self.fresh_for
	}
}

/// A recorded fetch failure. Backoff can end while the record persists; it is
/// cleared only by a successful fetch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
/// Three variants ask the pipeline's refresher (#446) for a refresh:
/// [`Self::ServeFreshAndRequestRefresh`], [`Self::RequestRefresh`] and
/// [`Self::RequestRefreshAfterFailure`]. A refresh-ahead request from a fresh
/// value never makes the reader wait; what an inline refresher does with one
/// is #446's to define. For the other two, the refresher decides whether the
/// reader waits. An inline refresher makes it wait for the fetch, and if the
/// fetch fails the reader gets the cached value only where
/// [`FreshnessPolicy::may_serve_fallback`] allows it, and the fetch's error
/// otherwise. A refresher that sidetracks the request, such as a Durable Object
/// alarm, lets the reader be answered with the cached value at once, as the
/// reference does.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum ReadAction<'a, T, E> {
	/// Serve the cached value as fresh. Fresh values are served even during
	/// backoff.
	ServeFresh {
		/// When the fetch that produced the cached value began.
		fetch_started_at: T,
	},
	/// Serve the cached value as fresh, and ask for a refresh because the value
	/// has reached its refresh point (see [`FreshnessPolicy::refresh_ahead`])
	/// and no backoff is active. The reader never waits for this refresh.
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

/// Written by hand because a derive would require `E: Clone`, although the
/// action only borrows the error.
impl<T: Copy, E> Clone for ReadAction<'_, T, E> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: Copy, E> Copy for ReadAction<'_, T, E> {}

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
	/// "Within the age limit" is inclusive, as in [`Self::may_serve_fallback`]:
	/// a value whose signed age equals [`Self::serve_cached_after_failure`] is
	/// within it, and every value is within no limit.
	///
	/// In order:
	/// 1. A current value younger than its freshness interval is served fresh,
	///    even during backoff. If its signed age has reached its refresh point
	///    (see [`Self::refresh_ahead`]) and no backoff is active, the read also
	///    asks for a refresh.
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
		let failure = entry.and_then(|entry| entry.failure.as_ref());
		let backing_off = failure.filter(|failure| failure.backoff_until > now);
		let Some(cached) = entry.and_then(|entry| entry.cached.as_ref()) else {
			return backing_off.map_or(ReadAction::FetchWithoutFallback, |failure| {
				ReadAction::ReportCachedError(&failure.last_error)
			});
		};

		let fetch_started_at = cached.fetch_started_at;
		let age = now.checked_duration_since(fetch_started_at);
		let fresh_for = cached.fresh_for.unwrap_or(self.fresh_for);
		let current = self.period_is_current(now, cached);
		let fresh = current && age.is_none_or(|age| age < fresh_for);
		let stale = current && age.is_none_or(|age| age < fresh_for.saturating_add(self.stale_window));
		let fallback = self.age_allows_fallback(age);
		match backing_off {
			None if fresh && self.reached_refresh_point(age, fresh_for) => {
				ReadAction::ServeFreshAndRequestRefresh { fetch_started_at }
			}
			_ if fresh => ReadAction::ServeFresh { fetch_started_at },
			Some(_) if fallback => ReadAction::ServeCachedAfterFailure { fetch_started_at },
			Some(failure) => ReadAction::ReportCachedError(&failure.last_error),
			None if stale && failure.is_none() => ReadAction::RequestRefresh { fetch_started_at },
			None if stale && fallback => ReadAction::RequestRefreshAfterFailure { fetch_started_at },
			None if fallback => ReadAction::FetchWithStaleFallback {
				fallback_fetch_started_at: fetch_started_at,
			},
			None => ReadAction::FetchWithoutFallback,
		}
	}

	/// What to record about a value from a successful fetch that began at
	/// `started_at`. `fresh_for` is the value's own freshness interval, if its
	/// source computes one. The period end is computed from `started_at`, so a
	/// fetch that crosses a period boundary belongs to the period it started in.
	/// This does not touch the failure record: the caller also sets
	/// [`Entry::failure`] to `None`.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L202-L209).
	///
	/// # Errors
	///
	/// [`Error::PeriodEndsBeforeFetch`] when [`Self::period_end`] returns a time
	/// at or before `started_at`. The pipeline treats that as a failed fetch, so
	/// backoff holds off the source and the faulty function is reported (#441).
	pub fn cached_after_success(
		&self,
		started_at: T,
		fresh_for: Option<Duration>,
	) -> Result<CachedValueMetadata<T>, Error> {
		CachedValueMetadata::new(
			started_at,
			self.period_end.map(|end| end(started_at)),
			fresh_for,
		)
	}

	/// Whether a kept value whose fetch began at `fallback_fetch_started_at` may
	/// be served after a failed fetch, at `now`: always without an age limit,
	/// otherwise when its signed age is at most the limit. Its period does not
	/// matter.
	#[must_use]
	pub fn may_serve_fallback(&self, now: T, fallback_fetch_started_at: T) -> bool {
		self.age_allows_fallback(now.checked_duration_since(fallback_fetch_started_at))
	}

	/// A hint for a refresher that schedules: when to set its next refresh, as
	/// the reference's `#schedule` sets its alarm after every fetch. A refresher
	/// may ignore it. An inline refresher never uses it.
	///
	/// `None` when the entry is idle under [`IDLE_AFTER`]: stop scheduling it.
	/// During backoff, when backoff ends: the earliest a retry is allowed.
	/// Otherwise, for a cached value, its fetch start plus the age of its refresh
	/// point (see [`Self::refresh_ahead`]), and without one, `now` plus
	/// [`Self::fresh_for`]. A time at or before `now` calls for checking
	/// eligibility now. [`Self::may_refresh`] still decides whether it can run.
	/// Period ends are not scheduled, as in the reference.
	///
	/// A time past the [`Timestamp`] type's ceiling comes back as the ceiling,
	/// which in practice is never reached. Only this hint computes a time;
	/// [`Self::on_read`] and [`Self::may_refresh`] compare ages, so they never
	/// disagree with each other at the ceiling.
	///
	/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L239-L249).
	#[must_use]
	pub fn next_refresh_hint<E>(&self, now: T, entry: &Entry<T, E>) -> Option<T> {
		if is_idle(now, entry.last_read) {
			return None;
		}
		if let Some(failure) = entry
			.failure
			.as_ref()
			.filter(|failure| failure.backoff_until > now)
		{
			return Some(failure.backoff_until);
		}
		Some(entry.cached.as_ref().map_or_else(
			|| now.saturating_add_duration(self.fresh_for),
			|cached| {
				cached
					.fetch_started_at
					.saturating_add_duration(self.refresh_point(cached.fresh_for.unwrap_or(self.fresh_for)))
			},
		))
	}

	/// Whether a refresher may run a refresh now. False when there is no entry,
	/// when it is idle under [`IDLE_AFTER`], during backoff, or while its current
	/// value has not reached its refresh point (see [`Self::refresh_ahead`]).
	/// True otherwise.
	///
	/// A refresh asked for by a read also passes this check, because a
	/// sidetracked refresh may run after backoff has begun. The entry given must
	/// already carry that read's time from [`record_read`], whether or not
	/// writing it to storage succeeded, as the reference writes `last_read`
	/// before it sets its alarm. Otherwise a read of an idle entry would ask for
	/// a refresh that this check then refuses, and an inline reader would wait
	/// for a fetch that never starts.
	///
	/// The reference's [`alarm()`][alarm] checks only idleness, but it is never
	/// set for a time when freshness or backoff would stop it. This check, agreed
	/// in #439, stops a refresher that runs early, such as a queue, from fetching
	/// too soon.
	///
	/// [alarm]: https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L251-L260
	#[must_use]
	pub fn may_refresh<E>(&self, now: T, entry: Option<&Entry<T, E>>) -> bool {
		entry.is_some_and(|entry| {
			!is_idle(now, entry.last_read)
				&& entry
					.failure
					.as_ref()
					.is_none_or(|failure| failure.backoff_until <= now)
				&& entry.cached.as_ref().is_none_or(|cached| {
					!self.period_is_current(now, cached)
						|| self.reached_refresh_point(
							now.checked_duration_since(cached.fetch_started_at),
							cached.fresh_for.unwrap_or(self.fresh_for),
						)
				})
		})
	}

	fn period_is_current(&self, now: T, cached: &CachedValueMetadata<T>) -> bool {
		self.period_end.is_none() || cached.period_ends_at.is_some_and(|end| end > now)
	}

	fn age_allows_fallback(&self, age: Option<Duration>) -> bool {
		self
			.serve_cached_after_failure
			.is_none_or(|limit| age.is_none_or(|age| age <= limit))
	}

	fn reached_refresh_point(&self, age: Option<Duration>, fresh_for: Duration) -> bool {
		age.is_some_and(|age| age >= self.refresh_point(fresh_for))
	}

	fn refresh_point(&self, fresh_for: Duration) -> Duration {
		if self.refresh_ahead < fresh_for {
			fresh_for.saturating_sub(self.refresh_ahead)
		} else {
			fresh_for
		}
	}
}

fn is_idle<T: Timestamp>(now: T, last_read: T) -> bool {
	now
		.checked_duration_since(last_read)
		.is_some_and(|age| age > IDLE_AFTER)
}

/// The time to record as the entry's last read, if any: `now` when there is no
/// entry, or when more than [`TOUCH_EVERY`] has passed since the recorded last
/// read. Otherwise `None`, including when the recorded time is after `now`.
/// The rule uses no source setting, so this is a function, not a policy method.
///
/// Record the read before acting on [`FreshnessPolicy::on_read`]'s answer, as
/// the reference does: a read of an idle entry makes it no longer idle, which
/// [`FreshnessPolicy::may_refresh`] relies on.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L92-L104).
#[must_use]
pub fn record_read<T: Timestamp, E>(now: T, entry: Option<&Entry<T, E>>) -> Option<T> {
	entry
		.is_none_or(|entry| {
			now
				.checked_duration_since(entry.last_read)
				.is_some_and(|age| age > TOUCH_EVERY)
		})
		.then_some(now)
}

/// The failure record after a fetch that began at `started_at` failed with
/// `error`, given the count of consecutive failures recorded before it.
///
/// The count is one more than `previous_failures`, or one without them,
/// clamped at `u32::MAX`. Backoff ends at `started_at` plus
/// [`backoff_delay`] for the new count, saturating at the time type's ceiling.
/// Pass the largest `retry_after` noted during the fetch, or zero when none
/// was sent. It is passed to [`backoff_delay`].
/// These rules use no source setting, so this is a function, not a policy
/// method.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L219-L232).
#[must_use]
pub fn failure_after<T: Timestamp, E>(
	started_at: T,
	previous_failures: Option<NonZeroU32>,
	error: E,
	retry_after: Duration,
) -> Failure<T, E> {
	let consecutive_failures =
		previous_failures.map_or(NonZeroU32::MIN, |count| count.saturating_add(1));
	Failure {
		consecutive_failures,
		backoff_until: started_at
			.saturating_add_duration(backoff_delay(consecutive_failures, retry_after)),
		last_error: error,
	}
}

/// How long a source is held off after its `consecutive_failures`-th failure in
/// a row, measured from the failed fetch's start.
///
/// The exponential part is `min(MIN_BACKOFF × 2^(n−1), MAX_BACKOFF)` for the
/// count `n`: 1,920 seconds at failure 7 and [`MAX_BACKOFF`] from failure 8,
/// computed without overflow. The delay is the greater of that and
/// `retry_after` capped at [`MAX_RETRY_AFTER`]. Any `retry_after` is accepted,
/// zero when the upstream sent none. The delay is never clamped by a time
/// type's ceiling, so it is the right value to report even when
/// [`Failure::backoff_until`] saturated.
///
/// [Reference](https://github.com/frog-pond/ccc-server/blob/54cf5862e4a09917e9bdc932fa1632fd1c3e0f49/workers/src/source-do.ts#L220-L225).
#[must_use]
pub fn backoff_delay(consecutive_failures: NonZeroU32, retry_after: Duration) -> Duration {
	let exponent = (consecutive_failures.get() - 1).min(7);
	let exponential = (MIN_BACKOFF * (1 << exponent)).min(MAX_BACKOFF);
	exponential.max(retry_after.min(MAX_RETRY_AFTER))
}
