//! Retrieval of native responses for prepared requests.

use std::error::Error;

use crate::future::BoxFuture;

/// Retrieves a response for an application-prepared request `P`.
///
/// The response goes to a [`crate::body::AdaptResponse`] implementation before
/// the application converts its body.
pub trait Source<P>: Send + Sync + 'static {
	/// The native response before adaptation, such as an HTTP response.
	type Response: Send + 'static;

	/// The integration's error for retrieving the response.
	type Error: Error + Send + Sync + 'static;

	/// Borrows the request while obtaining one response.
	///
	/// # Errors
	///
	/// Returns the source's error when the integration cannot obtain a response.
	fn retrieve<'a>(&'a self, request: &'a P) -> BoxFuture<'a, Result<Self::Response, Self::Error>>;
}
