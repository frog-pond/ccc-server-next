//! Shared fixture pipeline and failure event tests with in-memory upstreams.

use std::{
	future::Future,
	pin::pin,
	task::{Context, Poll, Waker},
};

use bytes::Bytes;
use fixture_experiment_route::{
	buffered_fixture_body, fixture_operation, Fixture, FixtureSourceError, InvalidNonSuccessStatus,
	NonSuccessStatus, RefreshFailure, RefreshFailureEvent,
};
use futures_util::stream;
use trifecta::{
	body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	source::Source,
};
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Clone, Copy)]
enum Sample {
	Body(&'static str),
	Split,
	UnusableStream,
	UnusableResponse,
	Status(u16),
	TransportFailure,
}

#[derive(Debug, thiserror::Error)]
#[error("simulated fixture I/O failure")]
struct FixtureIo;

struct SampleSource;

impl Source<Sample> for SampleSource {
	type Response = Sample;
	type Error = FixtureSourceError<FixtureIo>;

	fn retrieve<'a>(&'a self, sample: &'a Sample) -> BoxFuture<'a, Result<Sample, Self::Error>> {
		Box::pin(async {
			match *sample {
				Sample::Status(status) => Err(NonSuccessStatus::try_from(status).map_or_else(
					FixtureSourceError::InvalidStatus,
					FixtureSourceError::HttpStatus,
				)),
				Sample::TransportFailure => Err(FixtureSourceError::Fetch(FixtureIo)),
				other => Ok(other),
			}
		})
	}
}

struct SampleAdapter;

impl AdaptResponse<Sample> for SampleAdapter {
	type Metadata = ();
	type Error = FixtureIo;

	fn adapt_response(
		&self,
		response: Sample,
	) -> BoxFuture<'_, Result<BodyWithMetadata<()>, FixtureIo>> {
		Box::pin(async move {
			match response {
				Sample::Body(payload) => Ok(buffered_fixture_body(Bytes::from_static(
					payload.as_bytes(),
				))),
				Sample::Split => {
					let chunks = stream::iter([Ok(Bytes::from_static(b"4")), Ok(Bytes::from_static(b"2"))]);
					Ok(BodyWithMetadata::new(Body::new(Box::pin(chunks)), ()))
				}
				Sample::UnusableStream => {
					let chunks = stream::iter([Err(BodyReadError::new(FixtureIo))]);
					Ok(BodyWithMetadata::new(Body::new(Box::pin(chunks)), ()))
				}
				_ => Err(FixtureIo),
			}
		})
	}
}

#[allow(clippy::panic, reason = "A pending sample is a test harness failure")]
fn run(sample: Sample) -> Result<u32, RefreshFailure> {
	let operation = fixture_operation(SampleSource, SampleAdapter);
	let mut result = pin!(operation.perform(sample));
	match result
		.as_mut()
		.poll(&mut Context::from_waker(Waker::noop()))
	{
		Poll::Ready(value) => value.map_err(|error| error.failure),
		Poll::Pending => panic!("the fixture pipeline uses only ready test futures"),
	}
}

#[wasm_bindgen_test(unsupported = test)]
fn pipeline_decodes_one_or_more_chunks() {
	assert_eq!(run(Sample::Body("42")), Ok(42));
	assert_eq!(run(Sample::Split), Ok(42));
	assert_eq!(run(Sample::Body("-1")), Err(RefreshFailure::Decode));
	assert_eq!(run(Sample::Body("4294967296")), Err(RefreshFailure::Decode));
}

#[wasm_bindgen_test(unsupported = test)]
fn pipeline_preserves_source_failure_categories() -> Result<(), InvalidNonSuccessStatus> {
	let status = NonSuccessStatus::try_from(302)?;
	assert_eq!(
		run(Sample::Status(302)),
		Err(RefreshFailure::HttpStatus { status })
	);
	assert_eq!(run(Sample::Status(0)), Err(RefreshFailure::InvalidStatus));
	assert_eq!(run(Sample::TransportFailure), Err(RefreshFailure::Fetch));
	Ok(())
}

#[wasm_bindgen_test(unsupported = test)]
fn pipeline_preserves_body_failure_categories() {
	assert_eq!(run(Sample::UnusableResponse), Err(RefreshFailure::ReadBody));
	assert_eq!(run(Sample::UnusableStream), Err(RefreshFailure::ReadBody));
}

#[wasm_bindgen_test(unsupported = test)]
fn entry_points_share_the_refresh_event_format() -> serde_json::Result<()> {
	let event =
		RefreshFailureEvent::new(Fixture::Redirect, RefreshFailure::InvalidStatus, "status 0");
	assert_eq!(
		serde_json::to_string(&event)?,
		r#"{"event":"fixture_experiment.refresh_failed","fixture":"redirect","operation":"read_fixture","failure":{"kind":"invalid_status"},"cause":"status 0"}"#,
	);
	Ok(())
}
