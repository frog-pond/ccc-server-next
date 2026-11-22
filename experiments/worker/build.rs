//! Registers the available selectors for the opt-in WASM compiler probes.

fn main() {
	if std::env::var_os("CARGO_FEATURE_TOOLCHAIN_PROBES").is_some() {
		println!("cargo::rustc-check-cfg=cfg(worker_experiment_probe, values(\"bare_fetch\", \"bare_body\", \"reqwest_redirect\"))");
	}
}
