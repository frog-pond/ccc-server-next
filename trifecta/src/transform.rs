//! Application conversion of an adapted response into a domain value.

use std::error::Error;

use crate::{body::BodyWithMetadata, future::BoxFuture};

/// Converts a complete body and its metadata `M` into the application's `T`.
///
/// The transform can use the same prepared request `P` that retrieval used.
/// Its input shape is independent of the original response type.
pub trait Transform<P, M, T>: Send + Sync + 'static {
	/// The conversion error for responses that cannot produce a domain value.
	type Error: Error + Send + Sync + 'static;

	/// Consumes the adapted response and converts it into a domain value.
	///
	/// # Errors
	///
	/// Returns the transform's error if the response cannot be converted for
	/// this prepared request.
	fn transform<'a>(
		&'a self,
		request: &'a P,
		body: BodyWithMetadata<M>,
	) -> BoxFuture<'a, Result<T, Self::Error>>;
}
