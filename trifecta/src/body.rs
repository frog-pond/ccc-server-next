use std::{error::Error, fmt, pin::Pin};

use bytes::Bytes;
use futures_core::Stream;

use crate::future::BoxFuture;

/// A runtime-neutral stream of byte chunks.
pub type BoxByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, BodyReadError>> + Send + 'static>>;

/// An error encountered while consuming a [`Body`].
#[derive(Debug)]
pub struct BodyReadError {
	source: Box<dyn Error + Send + Sync>,
}

impl BodyReadError {
	pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
		Self {
			source: Box::new(error),
		}
	}
}

impl fmt::Display for BodyReadError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "body read failed: {}", self.source)
	}
}

impl Error for BodyReadError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(self.source.as_ref())
	}
}

/// A lazily consumed, runtime-neutral byte body.
pub struct Body {
	stream: BoxByteStream,
}

impl Body {
	#[must_use]
	pub fn new(stream: BoxByteStream) -> Self {
		Self { stream }
	}

	pub fn into_stream(self) -> BoxByteStream {
		self.stream
	}
}

/// A body paired with integration-defined metadata.
pub struct BodyWithMetadata<M> {
	body: Body,
	metadata: M,
}

impl<M> BodyWithMetadata<M> {
	pub fn new(body: Body, metadata: M) -> Self {
		Self { body, metadata }
	}

	pub fn body(&self) -> &Body {
		&self.body
	}

	pub fn metadata(&self) -> &M {
		&self.metadata
	}

	pub fn metadata_mut(&mut self) -> &mut M {
		&mut self.metadata
	}

	pub fn into_parts(self) -> (Body, M) {
		(self.body, self.metadata)
	}

	pub fn map_metadata<N>(self, map: impl FnOnce(M) -> N) -> BodyWithMetadata<N> {
		BodyWithMetadata::new(self.body, map(self.metadata))
	}
}

/// Converts a concrete source response into Trifecta's body representation.
pub trait AdaptResponse<R>: Send + Sync + 'static {
	type Metadata: Send + 'static;
	type Error: Error + Send + Sync + 'static;

	fn adapt_response(
		&self,
		response: R,
	) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>>;
}
