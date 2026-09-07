//! The definition of how adapted feed bytes become a domain value.

use std::{error::Error, fmt};

use futures_util::TryStreamExt as _;
use trifecta::{
	body::{BodyReadError, BodyWithMetadata},
	future::BoxFuture,
	transform::Transform,
};

use crate::domain::{Metadata, Request, WpJsonFeed};

#[derive(Debug)]
pub(super) enum WpJsonFeedTransformError {
	Body(BodyReadError),
	Utf8(std::string::FromUtf8Error),
}

impl fmt::Display for WpJsonFeedTransformError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Body(error) => write!(formatter, "could not read the feed body: {error}"),
			Self::Utf8(error) => write!(formatter, "feed was not UTF-8: {error}"),
		}
	}
}

impl Error for WpJsonFeedTransformError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Body(error) => Some(error),
			Self::Utf8(error) => Some(error),
		}
	}
}

/// An application implements [`Transform`] to define the one canonical path
/// from an adapted response body to its domain result.
///
/// If this operation later gains a raw cache, both cached bytes and newly
/// retrieved bytes will pass through this same transformation.
pub(super) struct WpJsonFeedTransform;

impl Transform<Request, Metadata, WpJsonFeed> for WpJsonFeedTransform {
	type Error = WpJsonFeedTransformError;

	fn transform<'a>(
		&'a self,
		_request: &'a Request,
		body: BodyWithMetadata<Metadata>,
	) -> BoxFuture<'a, Result<WpJsonFeed, Self::Error>> {
		Box::pin(async move {
			let (body, metadata) = body.into_parts();
			debug_assert_eq!(metadata.media_type, "application/json");

			let bytes = body
				.into_stream()
				.try_fold(Vec::new(), |mut bytes, chunk| async move {
					bytes.extend_from_slice(&chunk);
					Ok(bytes)
				})
				.await
				.map_err(WpJsonFeedTransformError::Body)?;
			let json = String::from_utf8(bytes).map_err(WpJsonFeedTransformError::Utf8)?;

			Ok(WpJsonFeed(json))
		})
	}
}
