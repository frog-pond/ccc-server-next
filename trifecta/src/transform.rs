use std::error::Error;

use crate::{body::BodyWithMetadata, future::BoxFuture};

/// Converts an adapted response body into an operation's canonical result.
pub trait Transform<P, M, T>: Send + Sync + 'static {
	type Error: Error + Send + Sync + 'static;

	fn transform<'a>(
		&'a self,
		request: &'a P,
		body: BodyWithMetadata<M>,
	) -> BoxFuture<'a, Result<T, Self::Error>>;
}
