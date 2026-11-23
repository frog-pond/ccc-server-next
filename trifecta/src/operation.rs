//! Invocation, stage failures, and the observations that accompany a value.

use std::{error::Error as StdError, fmt};

use crate::{future::BoxFuture, pipeline};

/// A terminal failure at one of the three execution stages.
///
/// Each variant holds the stage's error converted into the application's
/// operation error `E`. The cause is also available through the standard
/// error chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
	/// The source could not retrieve a response.
	Source(E),
	/// The adapter could not produce a complete body and its metadata.
	ResponseAdaptation(E),
	/// The transform could not produce a domain value.
	Transformation(E),
}

impl<E: fmt::Display> fmt::Display for Error<E> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		todo!()
	}
}

impl<E: StdError + 'static> StdError for Error<E> {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		todo!()
	}
}

/// How the execution path participated in caching.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheStatus {
	/// The operation used a pipeline configured without a cache.
	Disabled,
	/// The caller explicitly used the cache-free entry point.
	Bypassed,
}

/// How the invocation participated in singleflight coordination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlightStatus {
	/// The invocation ran independently.
	Independent,
}

/// A failure that the operation recovered from while producing a value.
///
/// An uncached pipeline has no recovery cases, so this enum starts empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToleratedFailure {}

/// Execution facts associated with a successful operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observations {
	pub(crate) cache_status: CacheStatus,
}

impl Observations {
	/// Returns how the invocation used the cache.
	#[must_use]
	pub fn cache_status(&self) -> CacheStatus {
		todo!()
	}

	/// Returns whether the invocation shared work with another caller.
	#[must_use]
	pub fn flight_status(&self) -> FlightStatus {
		todo!()
	}

	/// Returns failures the invocation recovered from, in encounter order.
	#[must_use]
	pub fn tolerated_failures(&self) -> &[ToleratedFailure] {
		todo!()
	}
}

/// A successful domain value and the observations from the same invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome<T> {
	pub(crate) value: T,
	pub(crate) observations: Observations,
}

impl<T> Outcome<T> {
	/// Borrows the domain value produced by the invocation.
	pub fn value(&self) -> &T {
		todo!()
	}

	/// Borrows the observations associated with this value.
	pub fn observations(&self) -> &Observations {
		todo!()
	}

	/// Consumes the outcome and returns both of its parts.
	pub fn into_parts(self) -> (T, Observations) {
		todo!()
	}

	/// Changes the value while carrying its observations into the new outcome.
	pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Outcome<U> {
		todo!()
	}
}

enum Dispatch<P, T, E> {
	Direct(pipeline::Handle<P, T, E>),
	Selected(Box<dyn pipeline::Selector<P, T, E>>),
}

/// An application operation from a prepared request `P` to domain value `T`.
///
/// Each stage converts its error into the operation's `E`. The application's
/// operation type stays the same when its chosen pipeline changes.
pub struct Operation<P, T, E> {
	dispatch: Dispatch<P, T, E>,
}

impl<P, T, E> Operation<P, T, E>
where
	P: Send + 'static,
	T: Send + 'static,
	E: StdError + Send + Sync + 'static,
{
	/// Sets up an operation with one pipeline for all requests.
	#[must_use]
	pub fn new(handle: pipeline::Handle<P, T, E>) -> Self {
		todo!()
	}

	/// Sets up an operation that selects an assembled pipeline per request.
	pub fn new_with_selector(selector: impl pipeline::Selector<P, T, E>) -> Self {
		todo!()
	}

	/// Runs the selected pipeline through its normal execution path.
	///
	/// # Errors
	///
	/// Returns the error of the first stage that fails, with its stage recorded.
	pub fn perform(&self, request: P) -> BoxFuture<'_, Result<Outcome<T>, Error<E>>> {
		todo!()
	}

	/// Runs only the selected pipeline's cache-free source, adapter, and transform.
	///
	/// # Errors
	///
	/// Returns the error of the first stage that fails, with its stage recorded.
	pub fn perform_bypassing_cache(&self, request: P) -> BoxFuture<'_, Result<Outcome<T>, Error<E>>> {
		todo!()
	}
}
