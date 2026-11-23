//! Assembly of a source, response adapter, and transform into a cache-free pipeline.

use std::error::Error;

use crate::{body::AdaptResponse, pipeline::Handle, source::Source, transform::Transform};

/// The three concrete stages before they are erased behind a [`Handle`].
pub struct Pipeline<S, A, X> {
	source: S,
	adapter: A,
	transform: X,
}

impl<S, A, X> Pipeline<S, A, X> {
	/// Collects the stages for type checking and erasure by [`Self::into_handle`].
	pub fn new(source: S, adapter: A, transform: X) -> Self {
		todo!()
	}

	/// Checks the types between stages and erases them into an operation handle.
	///
	/// The adapter must accept the source's response. The transform must accept
	/// the adapter's metadata. All three errors must convert into `E`.
	///
	/// A response type must match the adapter's input:
	///
	/// ```compile_fail,E0277
	/// use trifecta::{body::AdaptResponse, pipeline::Handle, source::Source, transform::Transform,
	///     uncached::Pipeline};
	/// use std::convert::Infallible;
	///
	/// fn assemble(
	///     source: impl Source<(), Response = u8, Error = Infallible>,
	///     adapter: impl AdaptResponse<String, Metadata = (), Error = Infallible>,
	///     transform: impl Transform<(), (), (), Error = Infallible>,
	/// ) -> Handle<(), (), Infallible> {
	///     Pipeline::new(source, adapter, transform).into_handle()
	/// }
	/// ```
	///
	/// The transform must accept the metadata the adapter produces:
	///
	/// ```compile_fail,E0277
	/// use trifecta::{body::AdaptResponse, pipeline::Handle, source::Source, transform::Transform,
	///     uncached::Pipeline};
	/// use std::convert::Infallible;
	///
	/// fn assemble(
	///     source: impl Source<(), Response = u8, Error = Infallible>,
	///     adapter: impl AdaptResponse<u8, Metadata = String, Error = Infallible>,
	///     transform: impl Transform<(), (), (), Error = Infallible>,
	/// ) -> Handle<(), (), Infallible> {
	///     Pipeline::new(source, adapter, transform).into_handle()
	/// }
	/// ```
	///
	/// The operation's error must accept each stage's error:
	///
	/// ```compile_fail,E0277
	/// use trifecta::{body::AdaptResponse, pipeline::Handle, source::Source, transform::Transform,
	///     uncached::Pipeline};
	/// use std::convert::Infallible;
	///
	/// fn assemble(
	///     source: impl Source<(), Response = u8, Error = Infallible>,
	///     adapter: impl AdaptResponse<u8, Metadata = (), Error = Infallible>,
	///     transform: impl Transform<(), (), (), Error = std::fmt::Error>,
	/// ) -> Handle<(), (), Infallible> {
	///     Pipeline::new(source, adapter, transform).into_handle()
	/// }
	/// ```
	pub fn into_handle<P, T, E>(self) -> Handle<P, T, E>
	where
		P: Send + Sync + 'static,
		T: Send + 'static,
		E: Error + Send + Sync + 'static + From<S::Error> + From<A::Error> + From<X::Error>,
		S: Source<P>,
		A: AdaptResponse<S::Response>,
		X: Transform<P, A::Metadata, T>,
	{
		todo!()
	}
}
