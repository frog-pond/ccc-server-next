//! Common pipeline identity, type erasure, and request-dependent selection.
//!
//! A concrete pipeline shape, such as [`crate::uncached::Pipeline`], checks and
//! assembles its stages before erasing them behind a cheap-to-clone [`Handle`].
//! Operations and selectors consequently depend only on the prepared request
//! `P` and result `T`, not on every configured source, cache, adapter, or
//! transform type.

use std::sync::Arc;

use crate::{future::BoxFuture, operation::Error};

/// A diagnostic identity for a configured pipeline.
///
/// The ID names an already assembled pipeline. It is not a registry lookup key
/// and does not select a pipeline by itself.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Id(Arc<str>);

impl Id {
	/// Creates an identity from an owned or shared string.
	pub fn new(value: impl Into<Arc<str>>) -> Self {
		Self(value.into())
	}

	/// Returns the identity as text.
	#[must_use]
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

pub(crate) trait Resolver<P, T>: Send + Sync + 'static {
	fn resolve(&self, request: P) -> BoxFuture<'_, Result<T, Error>>;
}

/// An opaque, completely assembled pipeline capability.
///
/// A handle contains separate normal and cache-bypass execution paths. For an
/// uncached pipeline both paths share the same resolver. Future cached handles
/// can give the bypass path a resolver that structurally owns no cache.
///
/// Handles are cheap to clone and are the only values a [`Selector`] may
/// return. They expose identity for diagnostics but execution remains owned by
/// [`crate::operation::Operation`].
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
	pub(crate) fn from_resolvers(
		id: Id,
		normal: Arc<dyn Resolver<P, T>>,
		bypass: Arc<dyn Resolver<P, T>>,
	) -> Self {
		Self { id, normal, bypass }
	}

	/// Returns this configured pipeline's diagnostic identity.
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
///
/// A selector owns or otherwise has access to a closed set of compatible
/// [`Handle`] values. Selection is asynchronous to permit application-defined
/// decision logic, but it is infallible: construct and validate every candidate
/// during application setup rather than during an invocation.
pub trait Selector<P, T>: Send + Sync + 'static {
	/// Selects the handle that will resolve `request`.
	fn select<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Handle<P, T>>;
}
