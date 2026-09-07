use std::{error::Error as StdError, fmt, sync::Arc};

use crate::{future::BoxFuture, pipeline};

/// The stable stage at which operation execution failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
	CacheRetrieval,
	CacheStorage,
	CacheRevalidation,
	Source,
	ResponseAdaptation,
	Transformation,
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
	Disabled,
	Bypassed,
}

/// A factual description of flight participation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FlightStatus {
	Independent,
}

/// A tolerated execution problem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Warning {
	message: Arc<str>,
}

impl Warning {
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

	#[must_use]
	pub fn cache_status(&self) -> CacheStatus {
		self.cache_status
	}

	#[must_use]
	pub fn flight_status(&self) -> FlightStatus {
		self.flight_status
	}

	pub fn warnings(&self) -> impl Iterator<Item = &Warning> {
		self.warnings.iter()
	}
}

/// A produced value and the factual observations from its execution.
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

	pub fn value(&self) -> &T {
		&self.value
	}

	pub fn value_mut(&mut self) -> &mut T {
		&mut self.value
	}

	pub fn observations(&self) -> &Observations {
		&self.observations
	}

	pub fn into_value(self) -> T {
		self.value
	}

	pub fn into_observations(self) -> Observations {
		self.observations
	}

	pub fn into_parts(self) -> (T, Observations) {
		(self.value, self.observations)
	}

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
pub struct Operation<P, T> {
	dispatch: Dispatch<P, T>,
}

impl<P, T> Operation<P, T>
where
	P: Send + 'static,
	T: Send + 'static,
{
	#[must_use]
	pub fn new(handle: pipeline::Handle<P, T>) -> Self {
		Self {
			dispatch: Dispatch::Direct(handle),
		}
	}

	pub fn new_with_selector(selector: impl pipeline::Selector<P, T>) -> Self {
		Self {
			dispatch: Dispatch::Selected(Arc::new(selector)),
		}
	}

	pub fn perform(&self, request: P) -> BoxFuture<'_, Result<Outcome<T>, Error>> {
		self.perform_with_cache_mode(request, false)
	}

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
