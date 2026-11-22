//! Retrieval of native values from external sources.
//!
//! A source is deliberately narrower than a complete pipeline: it obtains a
//! native response, while [`crate::body::AdaptResponse`] and
//! [`crate::transform::Transform`] handle the response afterward. Keeping the
//! native response intact leaves room for integrations that cache or inspect it
//! before it becomes a runtime-neutral [`crate::body::Body`].

use std::error::Error;

use crate::future::BoxFuture;

/// Acquires a native response for a prepared request.
///
/// `P` is prepared by the application before Trifecta is invoked. A source may
/// be an HTTP client, file reader, fixture store, database, or any other
/// asynchronous producer. Its [`Response`](Self::Response) remains a concrete
/// integration type and is converted by [`crate::body::AdaptResponse`].
pub trait Source<P>: Send + Sync + 'static {
	/// The source's native response, before response adaptation.
	type Response: Send + 'static;

	/// An error encountered while acquiring the native response.
	type Error: Error + Send + Sync + 'static;

	/// Retrieves one native response for `request`.
	///
	/// The request is borrowed only for the duration of the returned future.
	fn retrieve<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Result<Self::Response, Self::Error>>;
}
