use std::time::{SystemTime, UNIX_EPOCH};

use wasm_bindgen_test::wasm_bindgen_test;

use super::*;

type TestEntry = Entry<SystemTime, &'static str>;

fn at(seconds: u64) -> SystemTime {
	UNIX_EPOCH + Duration::from_secs(seconds)
}

fn policy() -> FreshnessPolicy<SystemTime> {
	FreshnessPolicy {
		fresh_for: Duration::from_hours(1),
		stale_window: Duration::from_hours(24),
		refresh_ahead: Duration::ZERO,
		serve_cached_after_failure: None,
		period_end: None,
	}
}

fn cached(policy: &FreshnessPolicy<SystemTime>, started_at: SystemTime) -> TestEntry {
	Entry {
		last_read: started_at,
		cached: Some(CachedValueMetadata {
			fetch_started_at: started_at,
			period_ends_at: policy.period_end.map(|end| end(started_at)),
			fresh_for: None,
		}),
		failure: None,
	}
}

fn fail(entry: &mut TestEntry, now: SystemTime) {
	let previous = entry.failure.map(|failure| failure.consecutive_failures);
	entry.failure = Some(failure_after(now, previous, "boom", Duration::ZERO));
}

#[wasm_bindgen_test(unsupported = test)]
fn cold_miss_fetches_and_then_serves_fresh() {
	let policy = policy();
	let started = at(100);
	assert_eq!(record_read::<_, &str>(started, None), Some(started));
	assert_eq!(
		policy.on_read::<&str>(started, None),
		ReadAction::FetchWithoutFallback
	);
	let mut entry = cached(&policy, started);
	entry.cached = policy.cached_after_success(started, None).ok();
	assert_eq!(
		policy.on_read(started, Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: started
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn read_within_ttl_serves_fresh_without_refresh() {
	let policy = policy();
	assert_eq!(
		policy.on_read(at(59 * 60), Some(&cached(&policy, at(0)))),
		ReadAction::ServeFresh {
			fetch_started_at: at(0)
		},
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn stale_read_can_serve_cached_value_while_requesting_refresh() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	let now = at(3_660);
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::RequestRefresh {
			fetch_started_at: at(0)
		}
	);
	assert_eq!(record_read(now, Some(&entry)), Some(now));
	entry.last_read = now;
	assert!(policy.may_refresh(now, Some(&entry)));
	entry.cached = policy.cached_after_success(now, None).ok();
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: now
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn scheduled_refresh_runs_at_expiry_and_schedules_the_next_one() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	assert_eq!(policy.next_refresh_hint(at(0), &entry), Some(at(3_600)));
	assert!(!policy.may_refresh(at(3_599), Some(&entry)));
	assert!(policy.may_refresh(at(3_600), Some(&entry)));
	entry.cached = policy.cached_after_success(at(3_600), None).ok();
	assert_eq!(policy.next_refresh_hint(at(3_600), &entry), Some(at(7_200)));
	assert_eq!(
		policy.on_read(at(3_600), Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: at(3_600)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn failure_without_a_value_reports_error_during_backoff() {
	let policy = policy();
	let mut entry = Entry {
		last_read: at(0),
		cached: None,
		failure: None,
	};
	assert_eq!(
		policy.on_read(at(0), Some(&entry)),
		ReadAction::FetchWithoutFallback
	);
	fail(&mut entry, at(0));
	assert_eq!(
		policy.on_read(at(1), Some(&entry)),
		ReadAction::ReportCachedError(&"boom")
	);
	assert_eq!(policy.next_refresh_hint(at(1), &entry), Some(at(30)));
}

#[wasm_bindgen_test(unsupported = test)]
fn failed_refresh_within_stale_window_serves_a_stand_in() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	let now = at(3_660);
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::RequestRefresh {
			fetch_started_at: at(0)
		}
	);
	fail(&mut entry, now);
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
	assert_eq!(
		policy.on_read(at(3_690), Some(&entry)),
		ReadAction::RequestRefreshAfterFailure {
			fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn failed_fetch_past_stale_window_can_serve_the_stored_value() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	let now = at(90_060);
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::FetchWithStaleFallback {
			fallback_fetch_started_at: at(0)
		}
	);
	assert!(policy.may_serve_fallback(now, at(0)));
	fail(&mut entry, now);
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn backoff_blocks_fetch_and_refresh_until_its_deadline() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	let now = at(90_060);
	entry.last_read = now;
	fail(&mut entry, now);
	assert!(!policy.may_refresh(at(90_089), Some(&entry)));
	assert_eq!(
		policy.on_read(at(90_089), Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
	assert!(policy.may_refresh(at(90_090), Some(&entry)));
	assert_eq!(
		policy.on_read(at(90_090), Some(&entry)),
		ReadAction::FetchWithStaleFallback {
			fallback_fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn each_consecutive_failure_doubles_backoff_until_the_cap() {
	let started = at(5);
	let mut previous = None;
	for seconds in [30, 60, 120, 240, 480, 960, 1_920, 3_600, 3_600] {
		let failure = failure_after(started, previous, "boom", Duration::ZERO);
		assert_eq!(
			failure.backoff_until,
			started + Duration::from_secs(seconds)
		);
		previous = Some(failure.consecutive_failures);
	}
	assert_eq!(previous, NonZeroU32::new(9));
}

fn period_policy() -> FreshnessPolicy<SystemTime> {
	FreshnessPolicy {
		period_end: Some(|started| started + Duration::from_secs(60)),
		..policy()
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn an_earlier_period_is_not_served_as_fresh_or_stale() {
	let policy = period_policy();
	let entry = cached(&policy, at(0));
	assert_eq!(
		policy.on_read(at(59), Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: at(0)
		}
	);
	assert_eq!(
		policy.on_read(at(60), Some(&entry)),
		ReadAction::FetchWithStaleFallback {
			fallback_fetch_started_at: at(0)
		}
	);
	assert!(policy.may_refresh(at(60), Some(&entry)));
}

#[wasm_bindgen_test(unsupported = test)]
fn an_earlier_period_remains_a_fallback_after_failure() {
	let policy = period_policy();
	let mut entry = cached(&policy, at(0));
	fail(&mut entry, at(60));
	assert_eq!(
		policy.on_read(at(60), Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn an_unread_entry_goes_quiet_after_two_days() {
	let policy = policy();
	let entry = cached(&policy, at(0));
	let idle_boundary = at(0) + IDLE_AFTER;
	assert!(policy.may_refresh(idle_boundary, Some(&entry)));
	let idle = idle_boundary + Duration::from_millis(1);
	assert!(!policy.may_refresh(idle, Some(&entry)));
	assert_eq!(policy.next_refresh_hint(idle, &entry), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn default_policy_serves_a_cached_value_at_any_age_after_failure() {
	let policy = policy();
	let mut entry = cached(&policy, at(0));
	let now = at(3_153_600_000);
	fail(&mut entry, now);
	assert!(policy.may_serve_fallback(now, at(0)));
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn age_limit_reports_error_only_past_the_limit_and_during_backoff() {
	let policy = FreshnessPolicy {
		serve_cached_after_failure: Some(Duration::from_mins(61)),
		..policy()
	};
	let mut entry = cached(&policy, at(0));
	assert_eq!(
		policy.on_read(at(3_661), Some(&entry)),
		ReadAction::RequestRefresh {
			fetch_started_at: at(0)
		}
	);
	fail(&mut entry, at(3_660));
	assert_eq!(
		policy.on_read(at(3_660), Some(&entry)),
		ReadAction::ServeCachedAfterFailure {
			fetch_started_at: at(0)
		}
	);
	assert!(!policy.may_serve_fallback(at(3_661), at(0)));
	assert_eq!(
		policy.on_read(at(3_661), Some(&entry)),
		ReadAction::ReportCachedError(&"boom")
	);
	assert_eq!(
		policy.on_read(at(3_690), Some(&entry)),
		ReadAction::FetchWithoutFallback
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn refresh_lead_uses_effective_lifetime_and_is_ignored_at_or_above_it() {
	let mut policy = FreshnessPolicy {
		refresh_ahead: Duration::from_secs(10),
		..policy()
	};
	let mut entry = cached(&policy, at(0));
	entry.cached = policy
		.cached_after_success(at(0), Some(Duration::from_mins(1)))
		.ok();
	assert_eq!(policy.next_refresh_hint(at(0), &entry), Some(at(50)));
	assert!(!policy.may_refresh(at(49), Some(&entry)));
	assert!(policy.may_refresh(at(50), Some(&entry)));
	assert_eq!(
		policy.on_read(at(50), Some(&entry)),
		ReadAction::ServeFreshAndRequestRefresh {
			fetch_started_at: at(0)
		}
	);
	fail(&mut entry, at(50));
	assert!(!policy.may_refresh(at(50), Some(&entry)));
	assert_eq!(
		policy.on_read(at(50), Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: at(0)
		}
	);
	entry.failure = None;
	for lead in [60, 600] {
		policy.refresh_ahead = Duration::from_secs(lead);
		assert_eq!(policy.next_refresh_hint(at(0), &entry), Some(at(60)));
		assert!(!policy.may_refresh(at(59), Some(&entry)));
		assert!(policy.may_refresh(at(60), Some(&entry)));
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn a_saturated_hint_does_not_make_refresh_eligible() {
	let policy = FreshnessPolicy {
		refresh_ahead: Duration::from_secs(30),
		..policy()
	};
	let ceiling = at(253_402_300_799);
	let started = ceiling - Duration::from_secs(10);
	let entry = cached(&policy, started);
	assert_eq!(policy.next_refresh_hint(ceiling, &entry), Some(ceiling));
	assert!(!policy.may_refresh(ceiling, Some(&entry)));
	assert_eq!(
		policy.on_read(ceiling, Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: started
		}
	);
	assert_eq!(started.saturating_add_duration(Duration::MAX), ceiling);
	let above = ceiling + Duration::from_secs(1);
	assert_eq!(above.saturating_add_duration(Duration::MAX), above);
	let failure = failure_after(started, None, "boom", Duration::ZERO);
	assert_eq!(failure.backoff_until, ceiling);
	assert_eq!(
		backoff_delay(failure.consecutive_failures, Duration::ZERO),
		MIN_BACKOFF
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn metadata_rejects_a_period_ending_at_or_before_fetch_start() {
	for end in [at(0), at(1)] {
		assert_eq!(
			CachedValueMetadata::new(at(1), Some(end), None),
			Err(Error::PeriodEndsBeforeFetch)
		);
	}
	let policy = FreshnessPolicy {
		period_end: Some(core::convert::identity),
		..policy()
	};
	assert_eq!(
		policy.cached_after_success(at(1), None),
		Err(Error::PeriodEndsBeforeFetch)
	);
	let metadata = period_policy().cached_after_success(at(1), Some(Duration::ZERO));
	assert_eq!(
		metadata.map(|value| (
			value.fetch_started_at(),
			value.period_ends_at(),
			value.fresh_for()
		)),
		Ok((at(1), Some(at(61)), Some(Duration::ZERO))),
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn period_policy_treats_a_value_without_a_stored_end_as_expired() {
	let policy = period_policy();
	let entry = cached(&self::policy(), at(0));
	assert_eq!(
		policy.on_read(at(1), Some(&entry)),
		ReadAction::FetchWithStaleFallback {
			fallback_fetch_started_at: at(0)
		}
	);
	assert!(policy.may_refresh(at(1), Some(&entry)));
	let with_period = cached(&policy, at(0));
	assert_eq!(
		self::policy().on_read(at(60), Some(&with_period)),
		ReadAction::ServeFresh {
			fetch_started_at: at(0)
		}
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn recording_a_read_before_checking_refresh_wakes_an_idle_entry() {
	let policy = FreshnessPolicy {
		stale_window: IDLE_AFTER,
		..policy()
	};
	let mut entry = cached(&policy, at(0));
	assert_eq!(record_read(at(0) + TOUCH_EVERY, Some(&entry)), None);
	let now = at(0) + IDLE_AFTER + Duration::from_secs(1);
	assert!(!policy.may_refresh(now, Some(&entry)));
	assert_eq!(
		policy.on_read(now, Some(&entry)),
		ReadAction::RequestRefresh {
			fetch_started_at: at(0)
		}
	);
	assert_eq!(record_read(now, Some(&entry)), Some(now));
	entry.last_read = now;
	assert!(policy.may_refresh(now, Some(&entry)));
}

#[wasm_bindgen_test(unsupported = test)]
fn a_clock_set_back_makes_the_value_younger_than_zero() {
	let policy = FreshnessPolicy {
		fresh_for: Duration::ZERO,
		serve_cached_after_failure: Some(Duration::ZERO),
		..period_policy()
	};
	let entry = cached(&policy, at(100));
	assert_eq!(
		policy.on_read(at(99), Some(&entry)),
		ReadAction::ServeFresh {
			fetch_started_at: at(100)
		}
	);
	assert!(!policy.may_refresh(at(99), Some(&entry)));
	assert!(policy.may_refresh(at(100), Some(&entry)));
	assert!(policy.may_serve_fallback(at(99), at(100)));
	assert_eq!(record_read(at(99), Some(&entry)), None);
}

#[wasm_bindgen_test(unsupported = test)]
fn retry_after_is_bounded_and_the_failure_count_saturates() {
	assert_eq!(
		backoff_delay(NonZeroU32::MIN, Duration::from_secs(1)),
		MIN_BACKOFF
	);
	assert_eq!(
		backoff_delay(NonZeroU32::MIN, Duration::from_mins(5)),
		Duration::from_mins(5)
	);
	let failure = failure_after(at(0), Some(NonZeroU32::MAX), "boom", Duration::MAX);
	assert_eq!(failure.consecutive_failures, NonZeroU32::MAX);
	assert_eq!(failure.backoff_until, at(0) + MAX_RETRY_AFTER);
	assert_eq!(backoff_delay(NonZeroU32::MAX, Duration::ZERO), MAX_BACKOFF);
}

#[cfg(feature = "jiff")]
#[wasm_bindgen_test(unsupported = test)]
fn jiff_saturates_and_preserves_signed_subsecond_elapsed_time() {
	let epoch = jiff::Timestamp::UNIX_EPOCH;
	let later = epoch.saturating_add_duration(Duration::from_nanos(1));
	assert_eq!(
		later.checked_duration_since(epoch),
		Some(Duration::from_nanos(1))
	);
	assert_eq!(epoch.checked_duration_since(later), None);
	assert_eq!(
		epoch.saturating_add_duration(Duration::MAX),
		jiff::Timestamp::MAX
	);
}
