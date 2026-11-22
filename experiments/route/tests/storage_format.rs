use fixture_experiment_route::{
	Fixture, NonSuccessStatus, ObjectReply, RefreshFailure, RefreshJob, RefreshOutcome,
};
use serde::{de::DeserializeOwned, Serialize};

fn assert_storage_format<T>(value: &T, json: &str)
where
	T: Serialize + DeserializeOwned + Eq + std::fmt::Debug,
{
	assert_eq!(serde_json::to_string(value).unwrap(), json);
	assert_eq!(&serde_json::from_str::<T>(json).unwrap(), value);
}

#[test]
fn refresh_job_json_is_stable() {
	assert_storage_format(
		&RefreshJob::ReadFixture(Fixture::Value),
		r#"{"name":"read_fixture","parameters":"value"}"#,
	);
	assert_storage_format(
		&RefreshJob::ReadFixture(Fixture::Redirect),
		r#"{"name":"read_fixture","parameters":"redirect"}"#,
	);
}

#[test]
fn fixture_names_identify_the_same_parameters_on_each_boundary() {
	for (fixture, segment, object) in [
		(Fixture::Value, "value", "fixture-value"),
		(Fixture::Redirect, "redirect", "fixture-redirect"),
	] {
		assert_eq!(segment.parse(), Ok(fixture));
		assert_eq!(
			Fixture::from_route_path(&format!("/experiment/{segment}")),
			Ok(fixture)
		);
		assert_eq!(fixture.object_name(), object);
		assert_eq!(Fixture::from_object_name(object), Ok(fixture));
	}
	assert!(Fixture::from_route_path("/experiment/missing").is_err());
	assert!(Fixture::from_route_path("/experiment/value/extra").is_err());
}

#[test]
fn refresh_outcome_json_is_stable() {
	assert_storage_format(
		&RefreshOutcome::Value { value: 42 },
		r#"{"outcome":"value","value":42}"#,
	);
	let status = NonSuccessStatus::try_from(302).unwrap();
	for (failure, json) in [
		(RefreshFailure::Fetch, r#"{"kind":"fetch"}"#),
		(
			RefreshFailure::InvalidStatus,
			r#"{"kind":"invalid_status"}"#,
		),
		(RefreshFailure::FixtureSetup, r#"{"kind":"fixture_setup"}"#),
		(
			RefreshFailure::HttpStatus { status },
			r#"{"kind":"http_status","status":302}"#,
		),
		(RefreshFailure::ReadBody, r#"{"kind":"read_body"}"#),
		(RefreshFailure::Decode, r#"{"kind":"decode"}"#),
		(RefreshFailure::Pipeline, r#"{"kind":"pipeline"}"#),
	] {
		assert_storage_format(
			&RefreshOutcome::Failed { failure },
			&format!(r#"{{"outcome":"failed","failure":{json}}}"#),
		);
	}
}

#[test]
fn object_reply_json_is_stable() {
	for (reply, json) in [
		(ObjectReply::Pending, r#"{"state":"pending"}"#),
		(
			ObjectReply::from(RefreshOutcome::Value { value: 42 }),
			r#"{"state":"complete","result":{"outcome":"value","value":42}}"#,
		),
		(
			ObjectReply::from(RefreshOutcome::Failed {
				failure: RefreshFailure::FixtureSetup,
			}),
			r#"{"state":"complete","result":{"outcome":"failed","failure":{"kind":"fixture_setup"}}}"#,
		),
		(
			ObjectReply::StorageUnavailable,
			r#"{"state":"storage_unavailable"}"#,
		),
		(
			ObjectReply::AlarmSchedulingFailed,
			r#"{"state":"alarm_scheduling_failed"}"#,
		),
		(
			ObjectReply::StoredRecordInvalid,
			r#"{"state":"stored_record_invalid"}"#,
		),
	] {
		assert_eq!(serde_json::to_string(&reply).unwrap(), json);
	}
}

#[test]
fn failure_status_rejects_success_and_non_http_codes() {
	for status in [0, 99, 200, 204, 299, 600, u16::MAX] {
		assert!(NonSuccessStatus::try_from(status).is_err());
		let json = format!(r#"{{"kind":"http_status","status":{status}}}"#);
		assert!(serde_json::from_str::<RefreshFailure>(&json).is_err());
	}
	for status in [100, 199, 300, 302, 599] {
		let json = status.to_string();
		assert_storage_format(&NonSuccessStatus::try_from(status).unwrap(), &json);
	}
}
