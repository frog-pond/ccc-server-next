//! The definition of the uncached way this operation can be resolved.

use trifecta::pipeline::{self as trifecta_pipeline, Handle, Uncached};

use crate::{
	body::WpJsonResponseAdapter,
	domain::{Request, WpJsonFeed},
	source::StubWpJsonSource,
	transform::WpJsonFeedTransform,
	ApplicationResourceContainer,
};

/// A pipeline is a compatible source, response adapter, and transform assembled
/// into an opaque handle.
///
/// This function defines exactly one pipeline and gives it no cache. Rust
/// checks all of the types before [`Uncached::into_handle`] erases the concrete
/// pieces behind [`Handle`].
pub(super) fn uncached(resources: &ApplicationResourceContainer) -> Handle<Request, WpJsonFeed> {
	Uncached::new(
		StubWpJsonSource {
			source_label: resources.source_label.clone(),
		},
		WpJsonResponseAdapter,
		WpJsonFeedTransform,
	)
	.into_handle(trifecta_pipeline::Id::new("wp-json"))
}
