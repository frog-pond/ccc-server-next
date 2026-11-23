# Worker experiment

Use Rust 1.98 or newer with the `wasm32-unknown-unknown` target, Node.js, and
Python 3. The Worker package pins Wrangler 4.166.0 and worker-build 0.9.0.

The crates under `experiments/` belong to the #452 prototype. They do not set
the design for the production builds. They pin the Trifecta draft through a
Git dependency. Port or remove the experiment when Trifecta lands.

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
npm run worker-build
```

Setup and the first build can download tools from npm and GitHub, in addition
to Cargo dependencies. Warm the build before applying the loopback-only runtime
restriction below.

Start Wrangler with the guarded `npm run dev` command below. It uses the pinned
local Wrangler and disables telemetry, error reports, Miniflare's `cf.json`
download, and the banner's npm update check.

Wrangler uses the crate-local worker-build binary and the local `FIXTURES`
binding with its SQLite migration.
Send requests to `/experiment/value` and
`/experiment/redirect` on `127.0.0.1:18788` for native and `127.0.0.1:8787` for
the Worker. The Worker may first return a pending reply while its alarm runs.
The completed responses share HTTP 200 and an `application/json` body. The
value is 42 and the redirect reports `http_status` 302. The fixture log should
show no request to `/redirect-target`. An unknown fixture returns 404. A non-GET
request for a known fixture, including `PROPFIND`, returns 405 with `Allow: GET`.

Each completed attempt schedules another refresh in five seconds. The Worker
serves the cached reply during that refresh. A failed refresh preserves the last
good value and logs `worker_experiment.stale_value_served` when a read serves it.
Both builds share a five-second deadline covering the source, response
adapter, and transform.

## Loopback-only verification

On macOS, after warming the build, run these commands from `experiments/worker`.
The verification runs used this OS policy to permit loopback and Unix socket
connections and deny other outbound connections. The OS guard still allows DNS
lookups. The opt-outs apply within the process, independently of the OS policy.

```sh
mkdir -p .wrangler
rm -f node_modules/.mf/cf.json
loopback_dev() {
  CARGO_NET_OFFLINE=true WRANGLER_LOG_PATH="$PWD/.wrangler/logs" \
    sandbox-exec -p '(version 1)(allow default)(deny network-outbound)(allow network-outbound (remote ip "localhost:*") (remote unix-socket))' \
    npm run dev -- "$@"
}
state=$(mktemp -d "$PWD/.wrangler/manual.XXXXXX")
loopback_dev --persist-to "$state"
```

If Wrangler asks for a login or a feature requires a remote binding, stop and
ask the maintainer. The following source locations are from the pinned npm
installation. They show the opt-outs independently of the network restriction.

| Option | Source in `node_modules` |
| --- | --- |
| Wrangler 4.166.0 telemetry | `wrangler/wrangler-dist/cli.js` line 10009 reads `WRANGLER_SEND_METRICS`. Lines 55845–55852 give it precedence over the project setting and global consent. |
| Wrangler 4.166.0 error reports | `wrangler/wrangler-dist/cli.js` lines 10011–10014 declare `WRANGLER_SEND_ERROR_REPORTS`. Lines 119107–119115 return before reporting when it is false. The transport at line 119147 targets `platform.dash.cloudflare.com`. |
| Wrangler 4.166.0 banner | `wrangler/wrangler-dist/cli.js` lines 10082–10084 read `WRANGLER_HIDE_BANNER`. Lines 80561–80566 return before the npm update check when it is true. |
| Miniflare 5.20261119.0-beta, locked by Wrangler 4.166.0 | `miniflare/dist/src/index.js` lines 113816–113817 declare the URL and variable. Lines 113875–113905 make `false` select built-in metadata before any cache read or download. |

## Manual runtime checks

These checks were manual local integration checks, outside `cargo test`. A
local harness could automate them by controlling the fixture responses,
starting and stopping Wrangler with disposable persistence, and checking
the HTTP replies, SQLite rows and structured logs. Start each case with a new
state directory unless it explicitly reuses one. Use `loopback_dev` above
from the Worker crate directory.

### SQLite rows and job ownership

Request `http://127.0.0.1:8787/experiment/value` once to create the object. Its
database is in
`$state/v3/do/ccc-fixture-experiment-FixtureObject/<actor-id>.sqlite`.
Skip `metadata.sqlite`, which holds the alarm scheduler's state. Identify the
value database by its stored job.

```sh
for db in "$state"/v3/do/ccc-fixture-experiment-FixtureObject/*.sqlite; do
  [ "${db##*/}" = metadata.sqlite ] && continue
  echo "$db"
  sqlite3 -readonly "$db" \
    'SELECT name, payload FROM fixture_records;
     SELECT state, refresh_after_ms, payload FROM fixture_cache;'
done
```

For the wrong-fixture check, stop Wrangler. Set `db` to the value object's
database and replace its job in the disposable state:

```sh
sqlite3 "$db" <<'SQL'
UPDATE fixture_records
SET payload = '{"name":"read_fixture","parameters":"redirect"}'
WHERE name = 'job';
SQL
```

Restart with the same state and request `/experiment/value`. Expect 500 with
`{"state":"stored_record_invalid"}` and an object failure event with
`operation: validate_job`.

### Invalid origin and missing binding

Start with new state and pass
`--var EXPERIMENT_FIXTURE_ORIGIN:https://127.0.0.1:18787` to `loopback_dev`.
Request `/experiment/value` and poll past the initial 202. Expect HTTP 200
with a completed `fixture_setup` failure and the matching refresh event.

For a missing binding, warm the normal Worker build first. Create this local
configuration in the ignored state directory and start a separate run:

```sh
mkdir -p .wrangler/missing-binding
cat > .wrangler/missing-binding/wrangler.toml <<'TOML'
name = "fixture-missing-binding"
main = "../../build/index.js"
compatibility_date = "2026-11-19"

[dev]
ip = "127.0.0.1"
port = 8787
TOML
loopback_dev --config .wrangler/missing-binding/wrangler.toml \
  --persist-to "$(mktemp -d "$PWD/.wrangler/missing.XXXXXX")"
```

Request `/experiment/value`. Expect 503 with `{"state":"object_call_failed"}`
and a `worker_experiment.object_call_failed` event for `resolve_binding`.

### Upstream 503 and stalled transfer

For the stale check, start with the normal fixture server, request
`/experiment/value`, and wait for 42. Stop the fixture server and replace it
with the following local fixture in `503` mode. For the deadline check, start
with empty Worker state and run the replacement fixture in `stall` mode
instead. Leave the Worker running during each check.

```sh
python3 - 503 <<'PYCODE'
from http.server import ThreadingHTTPServer
from runpy import run_path
import sys
import threading
import time

mode = sys.argv[1]
base = run_path("../fixtures.py")["FixtureHandler"]

class Fixture(base):
    attempts = 0
    lock = threading.Lock()

    def do_GET(self):
        if self.path != "/value":
            return super().do_GET()
        if mode == "503":
            self.send_error(503)
            return
        with self.lock:
            Fixture.attempts += 1
            attempt = Fixture.attempts
        if attempt == 1:
            time.sleep(20)
        elif attempt == 2:
            self.send_response(200)
            self.send_header("Content-Length", "2")
            self.end_headers()
            self.wfile.write(b"4")
            self.wfile.flush()
            time.sleep(20)
        else:
            super().do_GET()

with ThreadingHTTPServer(("127.0.0.1", 18787), Fixture) as server:
    server.serve_forever()
PYCODE
```

In `503` mode, wait for a refresh failure event with upstream status 503,
then read `/experiment/value`. Expect 42, a stale-serving event, and
`fixture_cache.state = 'stale'`. Restore the normal fixture to observe recovery.

In `stall` mode, the first request stalls before headers and the next stalls
after the first body byte. Both should log the typed `timeout` failure after
five seconds. With empty state the completed reply includes that failure.
The third alarm should store 42.

### Restart before the alarm

With a fresh state directory, request `/experiment/value` and stop Wrangler
after the 202 and before the three-second initial alarm fires. Query SQLite
as above to confirm the job exists and `fixture_cache` is empty. Restart
Wrangler with the same persistence directory. Query SQLite again before
making another HTTP request. The alarm should store the completed reply on
its own. The first GET after the write should return 42.

## Compile probes

From the repository root, check manual redirect setup and both wrapped futures
against the pinned Trifecta draft's actual `BoxFuture` type. The probes are an
opt-in example behind `toolchain-probes`. Normal Worker builds and CI build the
library. The explicit compiler selectors below intentionally fail one at a time.

```sh
cargo check --locked --target wasm32-unknown-unknown -p worker-experiment --features toolchain-probes --example wasm_probes
```

Run each of the following commands independently. The first two must fail to
compile with E0277 because the SDK futures are not `Send`. The third must fail
with E0433 and E0599 because reqwest's WASM client lacks the redirect-policy API.

```sh
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --features toolchain-probes --example wasm_probes -- --cfg 'worker_experiment_probe="bare_fetch"'
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --features toolchain-probes --example wasm_probes -- --cfg 'worker_experiment_probe="bare_body"'
cargo rustc --locked --target wasm32-unknown-unknown -p worker-experiment --features toolchain-probes --example wasm_probes -- --cfg 'worker_experiment_probe="reqwest_redirect"'
```

## Test and lint coverage

`mise run check` checks workspace formatting and runs host Clippy, Rust tests,
and board tool tests. The shared route tests check fixture naming, stable JSON,
status construction, body decoding, failure classification, and the common
event fields. On pull requests, GitHub Actions builds and tests the native
server and checks its value and redirect replies against the loopback fixture.
A separate job builds the Worker with pinned worker-build and lints its WASM
library. It runs without Wrangler and needs no Cloudflare resources.

Lint the WASM-only runtime separately:

```sh
cargo clippy --locked --target wasm32-unknown-unknown -p worker-experiment -p fixture-experiment-route --lib
```

Actual redirect handling, SQLite persistence, and alarm delivery require
`npm run dev` with the local-only settings above.

## Differences from Cloudflare's docs

Wrangler 4.166.0's `dev` command does not honor the project's `send_metrics`
setting for telemetry. The initial runs sent eight telemetry events to
`sparrow.cloudflare.com` and Miniflare fetched `cf.json` from
`workers.cloudflare.com`. The dev script sets the environment overrides above. The repeat runs
started with the cached `cf.json` removed.

Hosted Workers Fetch requires a hostname for subrequests, so the numeric
loopback upstream is specific to the local experiment. The compatibility date
is 2026-11-19, supported by the locked workerd 1.20261119.1.
[Cloudflare Workers known issues](https://developers.cloudflare.com/workers/platform/known-issues/#fetch-to-ip-addresses)

Cloudflare documents that a named object keeps its name on alarm delivery,
including when the alarm wakes it without a request. Alarms created before
2026-03-15 can lack that name. The alarm uses the stored fixture either way
and checks it against the object name when present.
[Durable Object ID documentation](https://developers.cloudflare.com/durable-objects/api/id/#name)
