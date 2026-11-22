//! A pipeline shape that always retrieves from its source.
//!
//! An uncached [`Pipeline`] composes source retrieval, native-response
//! adaptation, and canonical transformation without cache access. It is a peer
//! of future pre-parse and post-parse cached pipeline shapes; all of those shapes
//! erase into the common [`crate::pipeline::Handle`] consumed by an operation.

use std::{marker::PhantomData, sync::Arc};

use crate::{
	body::AdaptResponse,
	future::BoxFuture,
	operation::{Error, ErrorKind},
	pipeline::{Handle, Id, Resolver},
	source::Source,
	transform::Transform,
};

/// A typed source/response-adapter/transform composition with no cache.
///
/// This type exists only during application setup. It retains the concrete
/// stage types long enough for Rust to verify that:
///
/// - the response adapter accepts the source's native response;
/// - the transform accepts the adapter's metadata; and
/// - the transform produces the operation's result type.
///
/// Call [`Self::into_handle`] to erase the concrete types after that check.
pub struct Pipeline<U, N, X> {
	source: U,
	response_adapter: N,
	transform: X,
}

impl<U, N, X> Pipeline<U, N, X> {
	/// Assembles the three stages of an uncached pipeline.
	pub fn new(source: U, response_adapter: N, transform: X) -> Self {
		Self {
			source,
			response_adapter,
			transform,
		}
	}

	/// Type-checks this composition and erases it into an operation-ready handle.
	///
	/// `P` and `T` are normally inferred from the operation or function return
	/// type. Normal and bypass execution are behaviorally identical because this
	/// pipeline has no cache; their observations still differ at the operation
	/// boundary.
	pub fn into_handle<P, T>(self, id: Id) -> Handle<P, T>
	where
		P: Send + Sync + 'static,
		T: Send + 'static,
		U: Source<P>,
		N: AdaptResponse<U::Response>,
		X: Transform<P, N::Metadata, T>,
	{
		let resolver: Arc<dyn Resolver<P, T>> = Arc::new(UncachedResolver {
			source: self.source,
			response_adapter: self.response_adapter,
			transform: self.transform,
			marker: PhantomData,
		});

		Handle::from_resolvers(id, resolver.clone(), resolver)
	}
}

struct UncachedResolver<P, T, U, N, X> {
	source: U,
	response_adapter: N,
	transform: X,
	marker: PhantomData<fn(P) -> T>,
}

impl<P, T, U, N, X> Resolver<P, T> for UncachedResolver<P, T, U, N, X>
where
	P: Send + Sync + 'static,
	T: Send + 'static,
	U: Source<P>,
	N: AdaptResponse<U::Response>,
	X: Transform<P, N::Metadata, T>,
{
	fn resolve(&self, request: P) -> BoxFuture<'_, Result<T, Error>> {
		Box::pin(async move {
			let response = self
				.source
				.retrieve(&request)
				.await
				.map_err(|error| Error::from_source(ErrorKind::Source, error))?;
			let body = self
				.response_adapter
				.adapt_response(response)
				.await
				.map_err(|error| Error::from_source(ErrorKind::ResponseAdaptation, error))?;

			self
				.transform
				.transform(&request, body)
				.await
				.map_err(|error| Error::from_source(ErrorKind::Transformation, error))
		})
	}
}
