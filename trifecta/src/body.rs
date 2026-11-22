//! Runtime-neutral response bodies and native-response adaptation.
//!
//! [`Body`] contains only a stream of byte chunks. Protocol facts such as an
//! HTTP status or file modification time belong in the generic metadata carried
//! by [`BodyWithMetadata`]. Concrete integrations cross into this representation
//! by implementing [`AdaptResponse`].

use std::{error::Error, fmt, pin::Pin};

use bytes::Bytes;
use futures_core::Stream;

use crate::future::BoxFuture;

/// A runtime-neutral stream of byte chunks.
///
/// Each item is either the next owned chunk or an error encountered while
/// reading the underlying response. The stream owns everything it needs and
/// therefore has a `'static` lifetime.
pub type BoxByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, BodyReadError>> + Send + 'static>>;

/// An error encountered while consuming a [`Body`].
#[derive(Debug)]
pub struct BodyReadError {
	source: Box<dyn Error + Send + Sync>,
}

impl BodyReadError {
	/// Wraps an integration-specific body-reading error.
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
	/// Creates a body from an owned byte stream.
	#[must_use]
	pub fn new(stream: BoxByteStream) -> Self {
		Self { stream }
	}

	/// Returns the stream for one-time consumption.
	pub fn into_stream(self) -> BoxByteStream {
		self.stream
	}
}

/// A body paired with integration-defined metadata.
///
/// `M` may contain protocol-specific facts needed by a transform, such as an
/// HTTP status and selected headers. Use `()` when no metadata is needed.
pub struct BodyWithMetadata<M> {
	body: Body,
	metadata: M,
}

impl<M> BodyWithMetadata<M> {
	/// Pairs a runtime-neutral body with its metadata.
	pub fn new(body: Body, metadata: M) -> Self {
		Self { body, metadata }
	}

	/// Borrows the body without consuming it.
	///
	/// Reading the byte stream requires consuming `self` with [`Self::into_parts`].
	pub fn body(&self) -> &Body {
		&self.body
	}

	/// Borrows the integration-defined metadata.
	pub fn metadata(&self) -> &M {
		&self.metadata
	}

	/// Mutably borrows the integration-defined metadata.
	pub fn metadata_mut(&mut self) -> &mut M {
		&mut self.metadata
	}

	/// Separates the body and metadata for consumption.
	pub fn into_parts(self) -> (Body, M) {
		(self.body, self.metadata)
	}

	/// Changes the metadata type while preserving the body stream.
	pub fn map_metadata<N>(self, map: impl FnOnce(M) -> N) -> BodyWithMetadata<N> {
		BodyWithMetadata::new(self.body, map(self.metadata))
	}
}

/// Converts a concrete source response into Trifecta's body representation.
///
/// This stage captures useful native response facts in
/// [`BodyWithMetadata::metadata`] and converts the response payload into a
/// [`Body`]. It is separate from [`crate::source::Source`] so typed pipeline
/// integrations can act on the native response first, and separate from
/// [`crate::transform::Transform`] so every raw value follows one canonical
/// domain conversion.
pub trait AdaptResponse<R>: Send + Sync + 'static {
	/// Metadata retained from the native response.
	type Metadata: Send + 'static;

	/// An error encountered while adapting the native response.
	type Error: Error + Send + Sync + 'static;

	/// Converts one owned native response into a body and metadata.
	fn adapt_response(
		&self,
		response: R,
	) -> BoxFuture<'_, Result<BodyWithMetadata<Self::Metadata>, Self::Error>>;
}
