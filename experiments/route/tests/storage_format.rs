//! Stable JSON and route identifiers shared by the two entry points.

use fixture_experiment_route::{
	Fixture, InvalidNonSuccessStatus, NonSuccessStatus, ObjectReply, RefreshFailure, RefreshJob,
	RefreshOutcome,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen_test::wasm_bindgen_test;

#[allow(
	clippy::unwrap_used,
	reason = "Fail the test if the reply cannot be serialized"
)]
fn assert_storage_format(value: &impl Serialize, json: &str) {
	assert_eq!(serde_json::to_string(value).unwrap(), json);
}

#[wasm_bindgen_test(unsupported = test)]
fn refresh_job_json_is_stable() -> serde_json::Result<()> {
	let job: RefreshJob = serde_json::from_str(r#"{"name":"read_fixture","parameters":"redirect"}"#)?;
	assert_eq!(job, RefreshJob::ReadFixture(Fixture::Redirect));
	assert_storage_format(
		&RefreshJob::ReadFixture(Fixture::Value),
		r#"{"name":"read_fixture","parameters":"value"}"#,
	);
	assert_storage_format(
		&RefreshJob::ReadFixture(Fixture::Redirect),
		r#"{"name":"read_fixture","parameters":"redirect"}"#,
	);
	Ok(())
}

#[wasm_bindgen_test(unsupported = test)]
fn fixture_segments_identify_the_route_parameter() {
	for (fixture, segment) in [(Fixture::Value, "value"), (Fixture::Redirect, "redirect")] {
		assert_eq!(segment.parse(), Ok(fixture));
		assert_eq!(
			Fixture::from_route_path(&format!("/experiment/{segment}")),
			Ok(fixture)
		);
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn unknown_and_nested_fixture_paths_are_rejected() {
	assert!(Fixture::from_route_path("/experiment/missing").is_err());
	assert!(Fixture::from_route_path("/experiment/value/extra").is_err());
}

#[wasm_bindgen_test(unsupported = test)]
fn object_names_identify_the_fixture() {
	for (fixture, object) in [
		(Fixture::Value, "fixture-value"),
		(Fixture::Redirect, "fixture-redirect"),
	] {
		assert_eq!(fixture.object_name(), object);
		assert_eq!(Fixture::from_object_name(object), Ok(fixture));
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn refresh_outcome_json_is_stable() -> Result<(), InvalidNonSuccessStatus> {
	assert_storage_format(
		&RefreshOutcome::Value { value: 42 },
		r#"{"outcome":"value","value":42}"#,
	);
	for (failure, json) in failure_cases(NonSuccessStatus::try_from(302)?) {
		assert_storage_format(
			&RefreshOutcome::Failed { failure },
			&format!(r#"{{"outcome":"failed","failure":{json}}}"#),
		);
	}
	Ok(())
}

fn failure_cases(status: NonSuccessStatus) -> [(RefreshFailure, &'static str); 8] {
	[
		RefreshFailure::Fetch,
		RefreshFailure::Timeout,
		RefreshFailure::InvalidStatus,
		RefreshFailure::FixtureSetup,
		RefreshFailure::HttpStatus { status },
		RefreshFailure::ReadBody,
		RefreshFailure::Decode,
		RefreshFailure::Pipeline,
	]
	.map(|failure| {
		let json = match failure {
			RefreshFailure::Fetch => r#"{"kind":"fetch"}"#,
			RefreshFailure::Timeout => r#"{"kind":"timeout"}"#,
			RefreshFailure::InvalidStatus => r#"{"kind":"invalid_status"}"#,
			RefreshFailure::FixtureSetup => r#"{"kind":"fixture_setup"}"#,
			RefreshFailure::HttpStatus { .. } => r#"{"kind":"http_status","status":302}"#,
			RefreshFailure::ReadBody => r#"{"kind":"read_body"}"#,
			RefreshFailure::Decode => r#"{"kind":"decode"}"#,
			RefreshFailure::Pipeline => r#"{"kind":"pipeline"}"#,
		};
		(failure, json)
	})
}

#[wasm_bindgen_test(unsupported = test)]
fn serialized_failure_kinds_are_listed_in_the_sql_check() -> Result<(), Box<dyn std::error::Error>>
{
	#[derive(Deserialize)]
	struct FailureKind<'a> {
		kind: &'a str,
	}

	let sql = include_str!("../../worker/src/fixture_storage.sql");
	let check = sql
		.split_once("CONSTRAINT completed_failure CHECK (")
		.map_or("", |(_, check)| check);
	let kinds = check
		.split_once("json_extract(failure_payload, '$.result.failure.kind') IN (")
		.and_then(|(_, list)| list.split_once(") IS TRUE"))
		.map_or("", |(list, _)| list);
	for (failure, _) in failure_cases(NonSuccessStatus::try_from(302)?) {
		let json = serde_json::to_string(&failure)?;
		let FailureKind { kind } = serde_json::from_str(&json)?;
		let admitted = if let RefreshFailure::HttpStatus { .. } = failure {
			let branch = format!("WHEN '{kind}' THEN");
			check.lines().any(|line| line.trim() == branch)
		} else {
			let literal = format!("'{kind}'");
			kinds.split(',').any(|item| item.trim() == literal)
		};
		assert!(admitted, "completed_failure CHECK must admit {kind}");
	}
	Ok(())
}

#[wasm_bindgen_test(unsupported = test)]
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
		assert_storage_format(&reply, json);
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn object_call_failure_uses_the_object_reply_format() {
	assert_storage_format(
		&ObjectReply::ObjectCallFailed,
		r#"{"state":"object_call_failed"}"#,
	);
}

#[wasm_bindgen_test(unsupported = test)]
fn invalid_object_identity_uses_its_own_reply() {
	assert_storage_format(&ObjectReply::InvalidObject, r#"{"state":"invalid_object"}"#);
}

#[wasm_bindgen_test(unsupported = test)]
fn failure_status_rejects_success_and_non_http_codes() -> Result<(), InvalidNonSuccessStatus> {
	for status in [0, 99, 200, 204, 299, 600, u16::MAX] {
		assert!(NonSuccessStatus::try_from(status).is_err());
	}
	for status in [100, 199, 300, 302, 599] {
		let json = status.to_string();
		assert_storage_format(&NonSuccessStatus::try_from(status)?, &json);
	}
	Ok(())
}
