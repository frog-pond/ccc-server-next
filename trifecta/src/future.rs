//! Object-safe futures for pipeline stages, resolved by the caller's executor.

use std::{future::Future, pin::Pin};

/// A future that can move between threads on the native executor.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A local future for the Worker executor, including Workers Fetch futures.
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
