//! Registers the available selectors for the opt-in WASM compiler probes.

fn main() {
	if cfg!(feature = "toolchain-probes") {
		println!("cargo::rustc-check-cfg=cfg(worker_experiment_probe, values(\"bare_fetch\", \"bare_body\", \"reqwest_redirect\"))");
	}
}
