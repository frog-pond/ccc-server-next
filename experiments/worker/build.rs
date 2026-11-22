fn main() {
	println!("cargo::rustc-check-cfg=cfg(worker_experiment_probe, values(\"bare_fetch\", \"bare_body\", \"reqwest_redirect\"))");
}
