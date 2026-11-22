use serde::{de::DeserializeOwned, Serialize};
use worker_experiment::{Fixture, NonSuccessStatus, RefreshFailure, RefreshJob, RefreshOutcome};

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
fn refresh_outcome_json_is_stable() {
	assert_storage_format(
		&RefreshOutcome::Value { value: 42 },
		r#"{"outcome":"value","value":42}"#,
	);
	let status = NonSuccessStatus::try_from(302).unwrap();
	for (failure, json) in [
		(RefreshFailure::Fetch, r#"{"kind":"fetch"}"#),
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
