use std::{
	error::Error as _,
	fmt,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
};

use bytes::Bytes;
use futures_util::{future, stream};
use trifecta::{
	body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	operation::{CacheStatus, ErrorKind, Operation},
	pipeline::Id,
	source::Source,
	transform::Transform,
	uncached,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureStage {
	None,
	Source,
	ResponseAdaptation,
	Transformation,
}

#[derive(Debug)]
struct StageError(&'static str);

impl fmt::Display for StageError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(self.0)
	}
}

impl std::error::Error for StageError {}

struct TestSource {
	failure: FailureStage,
	calls: Arc<AtomicUsize>,
}

impl Source<()> for TestSource {
	type Error = StageError;
	type Response = Bytes;

	fn retrieve(&self, (): &()) -> BoxFuture<'_, Result<Self::Response, Self::Error>> {
		self.calls.fetch_add(1, Ordering::Relaxed);
		Box::pin(future::ready(if self.failure == FailureStage::Source {
			Err(StageError("source failed"))
		} else {
			Ok(Bytes::from_static(b"body"))
		}))
	}
}

struct TestResponseAdapter {
	failure: FailureStage,
	calls: Arc<AtomicUsize>,
}

impl AdaptResponse<Bytes> for TestResponseAdapter {
	type Error = StageError;
	type Metadata = ();

	fn adapt_response(
		&self,
		response: Bytes,
	) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>> {
		self.calls.fetch_add(1, Ordering::Relaxed);
		Box::pin(future::ready(
			if self.failure == FailureStage::ResponseAdaptation {
				Err(StageError("response adaptation failed"))
			} else {
				let chunks = stream::once(future::ready(Ok::<_, BodyReadError>(response)));
				Ok(BodyWithMetadata::new(Body::new(Box::pin(chunks)), ()))
			},
		))
	}
}

struct TestTransform {
	failure: FailureStage,
	calls: Arc<AtomicUsize>,
}

impl Transform<(), (), ()> for TestTransform {
	type Error = StageError;

	fn transform<'a>(
		&'a self,
		(): &'a (),
		_body: BodyWithMetadata<()>,
	) -> BoxFuture<'a, Result<(), Self::Error>> {
		self.calls.fetch_add(1, Ordering::Relaxed);
		Box::pin(future::ready(
			if self.failure == FailureStage::Transformation {
				Err(StageError("transformation failed"))
			} else {
				Ok(())
			},
		))
	}
}

struct Fixture {
	operation: Operation<(), ()>,
	source_calls: Arc<AtomicUsize>,
	adaptation_calls: Arc<AtomicUsize>,
	transform_calls: Arc<AtomicUsize>,
}

fn fixture(failure: FailureStage) -> Fixture {
	let source_calls = Arc::new(AtomicUsize::new(0));
	let adaptation_calls = Arc::new(AtomicUsize::new(0));
	let transform_calls = Arc::new(AtomicUsize::new(0));
	let handle = uncached::Pipeline::new(
		TestSource {
			failure,
			calls: source_calls.clone(),
		},
		TestResponseAdapter {
			failure,
			calls: adaptation_calls.clone(),
		},
		TestTransform {
			failure,
			calls: transform_calls.clone(),
		},
	)
	.into_handle(Id::new("test"));

	Fixture {
		operation: Operation::new(handle),
		source_calls,
		adaptation_calls,
		transform_calls,
	}
}

#[tokio::test]
async fn normal_and_bypass_have_distinct_cache_observations() {
	let fixture = fixture(FailureStage::None);

	let normal = fixture.operation.perform(()).await.unwrap();
	let bypass = fixture.operation.perform_bypassing_cache(()).await.unwrap();

	assert_eq!(normal.observations().cache_status(), CacheStatus::Disabled);
	assert_eq!(bypass.observations().cache_status(), CacheStatus::Bypassed);
	assert_eq!(fixture.source_calls.load(Ordering::Relaxed), 2);
	assert_eq!(fixture.adaptation_calls.load(Ordering::Relaxed), 2);
	assert_eq!(fixture.transform_calls.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn failures_retain_their_stage_source_and_short_circuit_later_stages() {
	let cases = [
		(FailureStage::Source, ErrorKind::Source, [1, 0, 0]),
		(
			FailureStage::ResponseAdaptation,
			ErrorKind::ResponseAdaptation,
			[1, 1, 0],
		),
		(
			FailureStage::Transformation,
			ErrorKind::Transformation,
			[1, 1, 1],
		),
	];

	for (failure, expected_kind, expected_calls) in cases {
		let fixture = fixture(failure);
		let error = fixture.operation.perform(()).await.unwrap_err();

		assert_eq!(error.kind(), expected_kind);
		assert!(error.source().is_some());
		assert_eq!(
			[
				fixture.source_calls.load(Ordering::Relaxed),
				fixture.adaptation_calls.load(Ordering::Relaxed),
				fixture.transform_calls.load(Ordering::Relaxed),
			],
			expected_calls
		);
	}
}
