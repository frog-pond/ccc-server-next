# Worker experiment

Use Rust 1.98 or newer and install the `wasm32-unknown-unknown` target. The
Worker entry points are draft signatures under review.

## Compile probes

From the repository root, check manual redirect setup and both wrapped futures
against the pinned Trifecta draft's actual `BoxFuture` type.

```sh
cargo check --locked --target wasm32-unknown-unknown -p worker-experiment --example wasm_probes
```

Run each of the following commands independently. The first two must fail to
compile with E0277 because the SDK futures are not `Send`. The third must fail
with E0433 and E0599 because reqwest's WASM client lacks the redirect-policy API.

```sh
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --example wasm_probes -- --cfg 'worker_experiment_probe="bare_fetch"'
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --example wasm_probes -- --cfg 'worker_experiment_probe="bare_body"'
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --example wasm_probes -- --cfg 'worker_experiment_probe="reqwest_redirect"'
```

`cargo test -p worker-experiment` fixes the JSON stored for jobs and outcomes
and checks the HTTP status type. The loopback redirect response, SQLite storage,
and alarm delivery will be checked in the Workers runtime after implementation.
