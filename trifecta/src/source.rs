use std::error::Error;

use crate::future::BoxFuture;

/// Acquires a native response for a prepared request.
pub trait Source<P>: Send + Sync + 'static {
	type Response: Send + 'static;
	type Error: Error + Send + Sync + 'static;

	fn retrieve<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Result<Self::Response, Self::Error>>;
}
