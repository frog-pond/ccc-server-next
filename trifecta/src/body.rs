//! The boundary between a source's response and the conversion that consumes it.

use std::error::Error;

use bytes::Bytes;

use crate::future::BoxFuture;

/// A complete response body paired with the facts the conversion needs.
///
/// The body owns its bytes. The integration supplies `M`, such as an HTTP
/// status and headers. Use `()` for responses with no additional facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyWithMetadata<M> {
	/// All bytes of the response, including an empty body when applicable.
	pub body: Bytes,
	/// Integration-defined response facts gathered during adaptation.
	pub metadata: M,
}

/// Buffers a source's native response and extracts the conversion's metadata.
pub trait AdaptResponse<R>: Send + Sync + 'static {
	/// Response facts to pass to the transform along with the bytes.
	type Metadata: Send + 'static;

	/// The integration's error for adapting the response.
	type Error: Error + Send + Sync + 'static;

	/// Consumes the native response and returns the complete body and metadata.
	///
	/// # Errors
	///
	/// Returns the adapter's error if it cannot read the complete body or
	/// produce the metadata it promises.
	fn adapt_response(
		&self,
		response: R,
	) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>>;
}
