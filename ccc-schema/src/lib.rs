//! Response types for the CCC API, shared by the native server and the Worker.
//!
//! Conversions construct these values, and the server serializes them for
//! the apps. The crate defines the app-facing schema and has no dependencies on
//! upstream readers, runtimes, or web frameworks.
//!
//! Each API area has its own module. A conversion imports only its own area's module,
//! and types used by multiple areas live in a shared module.

pub mod routing;

#[cfg(all(test, target_arch = "wasm32", target_os = "unknown"))]
mod tests {
	// Exercise the Wasm test runner on the shared schema crate.
	#[wasm_bindgen_test::wasm_bindgen_test]
	fn schema_test_harness_runs() {}
}
