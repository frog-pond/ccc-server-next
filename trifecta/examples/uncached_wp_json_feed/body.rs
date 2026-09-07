//! The definition of how a native response becomes a Trifecta body.

use std::convert::Infallible;

use bytes::Bytes;
use futures_util::stream;
use trifecta::{
	body::{AdaptResponse, Body, BodyReadError, BodyWithMetadata},
	future::BoxFuture,
};

use crate::domain::Metadata;

/// Adapts the source's response into Trifecta's runtime-neutral body and the
/// operation-specific [`Metadata`].
///
/// This boundary preserves useful native response facts without making
/// Trifecta depend on HTTP or another transport.
pub(super) struct WpJsonResponseAdapter;

impl AdaptResponse<Bytes> for WpJsonResponseAdapter {
	type Error = Infallible;
	type Metadata = Metadata;

	fn adapt_response(
		&self,
		response: Bytes,
	) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>> {
		Box::pin(async move {
			let chunks = stream::once(async move { Ok::<_, BodyReadError>(response) });
			Ok(BodyWithMetadata::new(
				Body::new(Box::pin(chunks)),
				Metadata {
					media_type: "application/json",
				},
			))
		})
	}
}
