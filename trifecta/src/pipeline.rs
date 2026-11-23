//! Assembled pipeline handles and application selection among them.

use std::sync::Arc;

use crate::{future::BoxFuture, operation::Error};

pub(crate) trait Fetch<P, T, E>: Send + Sync + 'static {
	fn fetch(&self, request: P) -> BoxFuture<'_, Result<T, Error<E>>>;
}

/// An assembled pipeline for prepared request `P`, value `T`, and error `E`.
///
/// Create one through [`crate::uncached::Pipeline::into_handle`], which checks
/// all three stages before erasing their concrete types. Clones share the
/// assembled stages.
pub struct Handle<P, T, E> {
	// This path owns only the source, adapter and transform. A cached shape can
	// add a separate normal execution path while keeping this path cache-free.
	pub(crate) fetch: Arc<dyn Fetch<P, T, E>>,
}

impl<P, T, E> Clone for Handle<P, T, E> {
	fn clone(&self) -> Self {
		todo!()
	}
}

/// Chooses an assembled pipeline from a set owned by the application.
pub trait Selector<P, T, E>: Send + Sync + 'static {
	/// Borrows the handle to use for the given prepared request.
	///
	/// Set up all choices before making the operation available to callers.
	fn select(&self, request: &P) -> &Handle<P, T, E>;
}
