use std::{marker::PhantomData, sync::Arc};

use crate::{
	body::AdaptResponse,
	future::BoxFuture,
	operation::{Error, ErrorKind},
	source::Source,
	transform::Transform,
};

/// A diagnostic identity for a configured pipeline.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Id(Arc<str>);

impl Id {
	pub fn new(value: impl Into<Arc<str>>) -> Self {
		Self(value.into())
	}

	#[must_use]
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

trait Resolver<P, T>: Send + Sync + 'static {
	fn resolve(&self, request: P) -> BoxFuture<'_, Result<T, Error>>;
}

/// An opaque, completely assembled pipeline capability.
pub struct Handle<P, T> {
	id: Id,
	normal: Arc<dyn Resolver<P, T>>,
	bypass: Arc<dyn Resolver<P, T>>,
}

impl<P, T> Clone for Handle<P, T> {
	fn clone(&self) -> Self {
		Self {
			id: self.id.clone(),
			normal: self.normal.clone(),
			bypass: self.bypass.clone(),
		}
	}
}

impl<P, T> Handle<P, T> {
	#[must_use]
	pub fn id(&self) -> &Id {
		&self.id
	}
}

impl<P, T> Handle<P, T>
where
	P: Send + 'static,
	T: Send + 'static,
{
	pub(crate) fn resolve(&self, request: P) -> BoxFuture<'_, Result<T, Error>> {
		self.normal.resolve(request)
	}

	pub(crate) fn resolve_bypassing_cache(&self, request: P) -> BoxFuture<'_, Result<T, Error>> {
		self.bypass.resolve(request)
	}
}

/// Selects one already assembled pipeline for a prepared request.
pub trait Selector<P, T>: Send + Sync + 'static {
	fn select<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Handle<P, T>>;
}

/// A typed, uncached source/response-adapter/transform composition.
pub struct Uncached<U, N, X> {
	source: U,
	response_adapter: N,
	transform: X,
}

impl<U, N, X> Uncached<U, N, X> {
	pub fn new(source: U, response_adapter: N, transform: X) -> Self {
		Self {
			source,
			response_adapter,
			transform,
		}
	}

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

		Handle {
			id,
			normal: resolver.clone(),
			bypass: resolver,
		}
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
