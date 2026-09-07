//! The definition of how this example retrieves a feed.

use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use trifecta::{future::BoxFuture, source::Source};

use crate::domain::Request;

/// An application implements [`Source`] for the external source it wants to
/// call. The associated `Response` is that source's native response type.
///
/// A real `WordPress` implementation might return a concrete HTTP response.
/// This fake returns `Bytes`, which keeps the example deterministic and avoids
/// requiring a network client.
pub(super) struct StubWpJsonSource {
	pub(super) source_label: Arc<str>,
}

impl Source<Request> for StubWpJsonSource {
	type Error = Infallible;
	type Response = Bytes;

	fn retrieve<'a>(
		&'a self,
		request: &'a Request,
	) -> BoxFuture<'a, Result<Self::Response, Self::Error>> {
		Box::pin(async move {
			Ok(Bytes::from(format!(
				r#"{{"site":"{}","source":"{}","posts":[]}}"#,
				request.site, self.source_label
			)))
		})
	}
}
