//! The application's values that move into and out of the operation.
//!
//! Trifecta does not prescribe an application's intent, prepared-request,
//! metadata, or domain-result types. This example gives each value a distinct
//! type so the point where it enters the pipeline remains visible.

/// The application decides what its callers are allowed to ask for.
///
/// A REST route could construct this "intent" from path and query parameters.
/// A CLI could construct the same type from command-line arguments. The service
/// receives it, but Trifecta does not.
pub(super) struct Intent {
	pub(super) site: String,
}

impl Intent {
	pub(super) fn new(site: &str) -> Self {
		Self {
			site: site.to_owned(),
		}
	}
}

/// The service turns an [`Intent`] into this prepared request.
///
/// Caller-local authorization and resource access would already be complete by
/// the time this value is passed to Trifecta.
pub(super) struct Request {
	pub(super) site: String,
}

/// An integration can attach arbitrary metadata to an adapted body.
///
/// An HTTP integration could retain a status code and selected response headers
/// here. The example only records the media type expected by its transform.
pub(super) struct Metadata {
	pub(super) media_type: &'static str,
}

/// The application decides which domain type the operation returns.
///
/// `WpJsonFeed` is not part of Trifecta. It is a small example domain type used
/// in place of a fully deserialized `WordPress` response model.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct WpJsonFeed(pub(super) String);

impl WpJsonFeed {
	pub(super) fn new(json: &str) -> Self {
		Self(json.to_owned())
	}
}
