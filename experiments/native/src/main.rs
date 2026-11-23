//! Native entry point for the fixture experiment.

#[cfg(not(target_arch = "wasm32"))]
mod server;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), server::NativeError> {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.map_err(server::NativeError::Runtime)?;
	runtime.block_on(server::run())
}

// The native listener is excluded from WASM workspace checks.
#[cfg(target_arch = "wasm32")]
fn main() {}
