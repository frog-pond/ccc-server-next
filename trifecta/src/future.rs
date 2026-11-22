//! Runtime-neutral asynchronous return types used by Trifecta traits.
//!
//! Trifecta uses a boxed future at its extension boundaries to keep those
//! boundaries object-safe without choosing an async runtime.

use std::{future::Future, pin::Pin};

/// A sendable, dynamically dispatched future.
///
/// Implementations normally return this with `Box::pin(async move { ... })`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
