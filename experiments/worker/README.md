# Worker experiment

Use Rust 1.98 or newer and install the `wasm32-unknown-unknown` target. The
native and Worker entry points are draft signatures under review.

## Local setup

From the repository root, run `python3 experiments/fixtures.py` to start the
fixture server at `127.0.0.1:18787`. `/value` returns `42` and `/redirect`
returns 302 with `Location: /redirect-target`. The target returns `99`, so a
followed redirect produces a visibly different success. The server logs every
request on stderr, including any request to the redirect target.

With the entry points implemented, run the native server from the root and the
Worker from its crate directory in separate terminals:

```sh
cargo run --locked -p fixture-experiment-native
```

```sh
cd experiments/worker
wrangler dev --local
```

Wrangler uses `worker-build` 0.9.0 on `PATH` and the local `FIXTURES` binding with
its SQLite class migration. Send requests to `/experiment/value` and
`/experiment/redirect` on `127.0.0.1:18788` for native and `127.0.0.1:8787` for
the Worker. The Worker may first return a pending reply while its alarm runs.
Use the same local persistence directory when restarting Wrangler to test that
an alarm can reload the job after restart. If Wrangler asks for a login or a
feature requires a remote binding, stop and ask the maintainer.

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

## Test and lint coverage

`mise run check` checks workspace formatting, runs host Clippy and tests, and runs the
board tool tests. The host Rust tests fix the JSON for stored jobs, outcomes,
and replies across the Durable Object call. They also check the HTTP status
type. Lint the WASM-only runtime and compile probes separately:

```sh
cargo clippy --locked --target wasm32-unknown-unknown -p worker-experiment --all-targets
```

Actual redirect handling, SQLite persistence, and alarm delivery require
`wrangler dev --local`.
