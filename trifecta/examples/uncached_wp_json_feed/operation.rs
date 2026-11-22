//! The definition of the callable, uncached `WpJsonFeed` operation.

use trifecta::operation::{self as trifecta_operation, Operation, Outcome};

use crate::{
	domain::{Intent, Request, WpJsonFeed},
	pipeline, ApplicationResourceContainer,
};

/// The application service owns request preparation and keeps the configured
/// [`Operation`] private.
///
/// Trifecta does not require an application to implement a service trait. This
/// ordinary Rust type is free to hold other application resources and perform
/// caller-local work before it invokes the operation.
pub(super) struct WpJsonService {
	feed: Operation<Request, WpJsonFeed>,
}

impl WpJsonService {
	#[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
	pub(super) async fn new(resources: &ApplicationResourceContainer) -> Self {
		// The service chooses the uncached pipeline during application setup.
		// There is no fallible or lazy configuration step on the first request.
		let feed = Operation::new(pipeline::uncached(resources));
		Self { feed }
	}

	pub(super) async fn load_feed(
		&self,
		intent: Intent,
	) -> Result<Outcome<WpJsonFeed>, trifecta_operation::Error> {
		// Preparation is application work. Only the prepared `Request` crosses
		// into Trifecta's potentially shareable execution pipeline.
		let request = Request { site: intent.site };
		self.feed.perform(request).await
	}
}
