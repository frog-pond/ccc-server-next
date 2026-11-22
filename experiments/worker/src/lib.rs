//! Workers runtime entry points for the local fixture route. The Worker forwards
//! the Durable Object's reply. The object's fetch handler manages the stored job
//! and alarm, and the alarm runs the shared route's Trifecta operation.

#[cfg(target_arch = "wasm32")]
mod runtime;
