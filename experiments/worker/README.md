# Worker experiment

Use Rust 1.98 or newer with the `wasm32-unknown-unknown` target, Node.js, and
Python 3. The Worker package pins Wrangler 4.166.0 and worker-build 0.9.0.

## Local setup

From the repository root, run `python3 experiments/fixtures.py` to start the
fixture server at `127.0.0.1:18787`. `/value` returns `42` and `/redirect`
returns 302 with `Location: /redirect-target`. The target returns `99`, so a
followed redirect produces a visibly different success. The server logs every
request on stderr, including any request to the redirect target.

Run the native server from the root and the Worker from its crate directory in
separate terminals:

```sh
cargo run --locked -p fixture-experiment-native
```

```sh
cd experiments/worker
npm ci
npm run worker-build:install
npm run dev
```

The `dev` script runs the following command. The environment flags disable
Wrangler telemetry and Miniflare's `cf.json` download for this process.

```sh
WRANGLER_SEND_METRICS=false CLOUDFLARE_CF_FETCH_ENABLED=false wrangler dev --local
```

Wrangler uses the crate-local worker-build binary and the local `FIXTURES`
binding with its SQLite migration.
Send requests to `/experiment/value` and
`/experiment/redirect` on `127.0.0.1:18788` for native and `127.0.0.1:8787` for
the Worker. The Worker may first return a pending reply while its alarm runs.
The completed responses share HTTP 200 and an `application/json` body. The
value is 42 and the redirect reports `http_status` 302. The fixture log should
show no request to `/redirect-target`. An unknown fixture returns 404. A non-GET
request for a known fixture returns 405 with `Allow: GET`.

To test alarm recovery, start with `npm run dev -- --persist-to <local-dir>`.
Get a pending reply, stop Wrangler before the three-second alarm delay elapses,
and restart with the same persistence directory. The alarm reloads the SQLite
job and stores the completed JSON reply even before another request arrives.
Each completed attempt stores a five-second refresh deadline and schedules
the next alarm. Reads serve the cached reply while the alarm refreshes it. If
the refresh fails, a previous good value stays cached. The Worker logs the
refresh failure and emits `worker_experiment.stale_value_served` on reads that
serve that value.
If Wrangler asks for a login or a feature requires a remote binding, stop and
ask the maintainer.

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

`mise run check` checks workspace formatting and runs host Clippy, Rust tests,
and board tool tests. The shared route tests check fixture naming, stable JSON,
status construction, body decoding, failure classification, and the common
event fields. Lint the WASM-only runtime and compile probes separately:

```sh
cargo clippy --locked --target wasm32-unknown-unknown -p worker-experiment --all-targets
```

Actual redirect handling, SQLite persistence, and alarm delivery require
`npm run dev` with the local-only settings above.

## Differences from Cloudflare's docs

Wrangler 4.166.0's `dev` command does not honor the project's `send_metrics`
setting for telemetry. The initial runs sent eight telemetry events to
`sparrow.cloudflare.com` and Miniflare fetched `cf.json` from
`workers.cloudflare.com`. The dev script now sets both environment overrides.
The repeat run started with the cached `cf.json` removed.
