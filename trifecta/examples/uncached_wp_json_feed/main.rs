//! Fetch a `WordPress` JSON feed with one uncached Trifecta operation.
//!
//! # A mental map of this example
//!
//! This file is the application entry point. It creates the application's shared
//! resources, constructs a service once, and calls that service with one caller's
//! intent. The rest of the example is divided using the same nouns that appear in
//! Trifecta's API:
//!
//! - [`domain`] defines caller intent, the prepared request, metadata, and the
//!   final `WpJsonFeed` domain value.
//! - [`body`] adapts a native source response into Trifecta's byte body.
//! - [`source`] defines how the prepared request fetches a native response. Its
//!   fake implementation keeps this example independent of HTTP and the network.
//! - [`transform`] defines the canonical conversion from the adapted byte body
//!   into the application's `WpJsonFeed` value.
//! - [`pipeline`] assembles the source, response adapter, and transform into one
//!   compatible, explicitly uncached pipeline.
//! - [`operation`] owns that pipeline behind the application's service. The
//!   service prepares caller intent and exposes the method the application calls.
//!
//! The swimlane below shows how data moves between those nouns:
//!
//! ```text
//! application    operation       pipeline        source        body        transform       domain
//!      |              |               |              |            |             |             |
//!      |-- Intent --->|               |              |            |             |             |
//!      |              | prepares domain::Request     |            |             |             |
//!      |              |-- Operation::perform ------->|            |             |             |
//!      |              |               |-- retrieve ->|            |             |             |
//!      |              |               |<- Response --|            |             |             |
//!      |              |               |--- AdaptResponse -------->|             |             |
//!      |              |               |<- BodyWithMetadata -------|             |             |
//!      |              |               |------- Transform ---------------------->|             |
//!      |              |               |<------------------------ WpJsonFeed ------------------|
//!      |              |<-- Outcome ---|              |            |             |             |
//!      |<- Outcome ---|               |              |            |             |             |
//! ```
//!
//! In runtime order: the service prepares a `Request`, `Operation::perform`
//! enters Trifecta, the pipeline invokes its `Source`, `AdaptResponse` produces
//! a body, `Transform` produces the domain value, and Trifecta returns an
//! `Outcome`.
//!
//! The application owns everything on either side of this pipeline: how caller
//! input becomes intent, which resources the service may use, and what happens to
//! the returned domain value. Trifecta owns the generic execution between the
//! prepared request and the outcome.

mod body;
mod domain;
mod operation;
mod pipeline;
mod source;
mod transform;

use std::{error::Error, sync::Arc, time::Instant};

use domain::{Intent, WpJsonFeed};
use operation::Service;
use trifecta::operation::CacheStatus;

/// The application can create arbitrary types for holding shared resources
/// together. Trifecta does not provide or require a resource container.
///
/// A production application might put HTTP clients, database pools, and
/// repositories here. This example only stores a label used by its fake
/// source.
struct ApplicationResourceContainer {
	source_label: Arc<str>,
}

impl ApplicationResourceContainer {
	fn new(source_label: &str) -> Self {
		Self {
			source_label: Arc::from(source_label),
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let resources = ApplicationResourceContainer::new("example-api");

	// The application creates its service once and decides whether setup should
	// be measured. Trifecta does not own this lifecycle or this clock.
	let setup_started = Instant::now();
	let feeds = Service::new(&resources).await;
	println!("service setup took {:?}", setup_started.elapsed());

	// A caller gives the application an "intent". The service will prepare that
	// intent before invoking the private operation assembled during setup.
	let invocation_started = Instant::now();
	let outcome = feeds.load_feed(Intent::new("blog.example.com")).await?;
	println!("service invocation took {:?}", invocation_started.elapsed());

	assert_eq!(
		outcome.value(),
		&WpJsonFeed::new(r#"{"site":"blog.example.com","source":"example-api","posts":[]}"#)
	);
	assert_eq!(outcome.observations().cache_status(), CacheStatus::Disabled);
	println!("feed: {:?}", outcome.into_value());

	Ok(())
}
