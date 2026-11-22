//! Canonical conversion from adapted bodies to domain values.
//!
//! A transform is source-blind: it receives the same
//! [`BodyWithMetadata`] whether a raw value came
//! from a source or, in a future cached pipeline, from a cache.

use std::error::Error;

use crate::{body::BodyWithMetadata, future::BoxFuture};

/// Converts an adapted response body into an operation's canonical result.
///
/// `P` is the prepared request, `M` is integration-defined response metadata,
/// and `T` is the domain value returned by the operation.
///
/// Any field of `P` that changes `T` must also be represented by the cache and
/// flight identities when those features are configured. Trifecta cannot infer
/// that relationship from the types alone.
pub trait Transform<P, M, T>: Send + Sync + 'static {
	/// An error encountered while reading or transforming the adapted body.
	type Error: Error + Send + Sync + 'static;

	/// Consumes an adapted body and produces the operation's domain value.
	fn transform<'a>(
		&'a self,
		request: &'a P,
		body: BodyWithMetadata<M>,
	) -> BoxFuture<'a, Result<T, Self::Error>>;
}
