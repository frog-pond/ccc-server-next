use std::{future::Future, pin::Pin};

/// A sendable, dynamically dispatched future.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
