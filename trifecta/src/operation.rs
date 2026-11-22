//! Invocation, errors, results, and factual execution observations.
//!
//! [`Operation`] is the application-facing Trifecta capability. It accepts an
//! already prepared request, dispatches it through a configured pipeline, and
//! returns an [`Outcome`] containing the domain value and non-timing facts.
//! Routing, authorization, setup, clocks, and presentation remain application
//! responsibilities.

use std::{error::Error as StdError, fmt, sync::Arc};

use crate::{future::BoxFuture, pipeline};

/// The stable stage at which operation execution failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
	/// Reading a configured cache failed.
	CacheRetrieval,
	/// Writing a configured cache failed.
	CacheStorage,
	/// Revalidating a cached value failed.
	CacheRevalidation,
	/// Acquiring a native response from the source failed.
	Source,
	/// Converting a native response to a runtime-neutral body failed.
	ResponseAdaptation,
	/// Converting an adapted body to the domain value failed.
	Transformation,
	/// Coordinating equivalent concurrent work failed.
	Flight,
}

/// An execution failure with a stable stage and its concrete source.
#[derive(Debug)]
pub struct Error {
	kind: ErrorKind,
	source: Box<dyn StdError + Send + Sync>,
}

impl Error {
	pub(crate) fn from_source(
		kind: ErrorKind,
		source: impl StdError + Send + Sync + 'static,
	) -> Self {
		Self {
			kind,
			source: Box::new(source),
		}
	}

	/// Returns the stable execution stage at which the operation failed.
	#[must_use]
	pub fn kind(&self) -> ErrorKind {
		self.kind
	}
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"operation failed during {:?}: {}",
			self.kind, self.source
		)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		Some(self.source.as_ref())
	}
}

/// A factual description of cache participation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CacheStatus {
	/// The selected pipeline has no cache.
	Disabled,
	/// The caller explicitly selected the cache-free execution path.
	Bypassed,
}

/// A factual description of flight participation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FlightStatus {
	/// This invocation was executed independently rather than coalesced.
	Independent,
}

/// A tolerated execution problem.
///
/// Warnings record failures that configured behavior allowed the operation to
/// recover from. They do not represent the terminal [`Error`] of an invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Warning {
	message: Arc<str>,
}

impl Warning {
	/// Returns the human-readable warning message.
	#[must_use]
	pub fn message(&self) -> &str {
		&self.message
	}
}

/// Non-timing facts recorded while performing an operation.
#[derive(Clone, Debug)]
pub struct Observations {
	cache_status: CacheStatus,
	flight_status: FlightStatus,
	warnings: Vec<Warning>,
}

impl Observations {
	pub(crate) fn uncached(bypassed: bool) -> Self {
		Self {
			cache_status: if bypassed {
				CacheStatus::Bypassed
			} else {
				CacheStatus::Disabled
			},
			flight_status: FlightStatus::Independent,
			warnings: Vec::new(),
		}
	}

	/// Describes how this invocation participated in caching.
	#[must_use]
	pub fn cache_status(&self) -> CacheStatus {
		self.cache_status
	}

	/// Describes how this invocation participated in singleflight coordination.
	#[must_use]
	pub fn flight_status(&self) -> FlightStatus {
		self.flight_status
	}

	/// Iterates over tolerated problems in encounter order.
	pub fn warnings(&self) -> impl Iterator<Item = &Warning> {
		self.warnings.iter()
	}
}

/// A produced value and the factual observations from its execution.
///
/// An outcome intentionally contains no timings. Applications decide which
/// setup, preparation, execution, or presentation boundaries to measure.
#[derive(Debug)]
pub struct Outcome<T> {
	value: T,
	observations: Observations,
}

impl<T> Outcome<T> {
	pub(crate) fn new(value: T, observations: Observations) -> Self {
		Self {
			value,
			observations,
		}
	}

	/// Borrows the produced domain value.
	pub fn value(&self) -> &T {
		&self.value
	}

	/// Mutably borrows the produced domain value.
	pub fn value_mut(&mut self) -> &mut T {
		&mut self.value
	}

	/// Borrows the execution observations.
	pub fn observations(&self) -> &Observations {
		&self.observations
	}

	/// Consumes the outcome and returns only its value.
	pub fn into_value(self) -> T {
		self.value
	}

	/// Consumes the outcome and returns only its observations.
	pub fn into_observations(self) -> Observations {
		self.observations
	}

	/// Consumes the outcome and separates its value and observations.
	pub fn into_parts(self) -> (T, Observations) {
		(self.value, self.observations)
	}

	/// Maps the produced value while preserving all observations.
	pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Outcome<U> {
		Outcome {
			value: map(self.value),
			observations: self.observations,
		}
	}
}

impl<T> AsRef<T> for Outcome<T> {
	fn as_ref(&self) -> &T {
		self.value()
	}
}

enum Dispatch<P, T> {
	Direct(pipeline::Handle<P, T>),
	Selected(Arc<dyn pipeline::Selector<P, T>>),
}

/// A configured domain-producing capability.
///
/// Construct an operation once during application setup and keep it behind an
/// application-specific service method. `P` is the prepared request accepted by
/// Trifecta, and `T` is the produced domain value.
pub struct Operation<P, T> {
	dispatch: Dispatch<P, T>,
}

impl<P, T> Operation<P, T>
where
	P: Send + 'static,
	T: Send + 'static,
{
	/// Creates an operation that always uses one assembled pipeline.
	#[must_use]
	pub fn new(handle: pipeline::Handle<P, T>) -> Self {
		Self {
			dispatch: Dispatch::Direct(handle),
		}
	}

	/// Creates an operation that selects among assembled pipelines per request.
	///
	/// The selector is invoked for both normal and bypassing calls. It cannot
	/// return an unassembled pipeline because its result is a [`pipeline::Handle`].
	pub fn new_with_selector(selector: impl pipeline::Selector<P, T>) -> Self {
		Self {
			dispatch: Dispatch::Selected(Arc::new(selector)),
		}
	}

	/// Performs the operation through its normal execution path.
	pub fn perform(&self, request: P) -> BoxFuture<'_, Result<Outcome<T>, Error>> {
		self.perform_with_cache_mode(request, false)
	}

	/// Performs the operation through its structurally cache-free path.
	///
	/// For an uncached pipeline this executes the same resolver as [`Self::perform`]
	/// but records [`CacheStatus::Bypassed`] rather than
	/// [`CacheStatus::Disabled`].
	pub fn perform_bypassing_cache(&self, request: P) -> BoxFuture<'_, Result<Outcome<T>, Error>> {
		self.perform_with_cache_mode(request, true)
	}

	fn perform_with_cache_mode(
		&self,
		request: P,
		bypassed: bool,
	) -> BoxFuture<'_, Result<Outcome<T>, Error>> {
		Box::pin(async move {
			let handle = match &self.dispatch {
				Dispatch::Direct(handle) => handle.clone(),
				Dispatch::Selected(selector) => selector.select(&request).await,
			};
			let value = if bypassed {
				handle.resolve_bypassing_cache(request).await?
			} else {
				handle.resolve(request).await?
			};

			Ok(Outcome::new(value, Observations::uncached(bypassed)))
		})
	}
}
