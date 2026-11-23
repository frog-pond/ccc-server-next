# Worker experiment

Use Rust 1.100 or newer with the `wasm32-unknown-unknown` target, Node.js, and
Python 3. The Worker package pins Wrangler 4.166.0 and worker-build 0.8.5.
The Rust SDK is pinned to workers-rs 0.8.5 so it can use the workspace's
wasm-bindgen 0.2.127 and wasm-bindgen-test 0.3.77.

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

The Worker uses `trifecta-freshness` to select each reply and when to run
its next alarm. The fixture policy makes a value fresh for five seconds from
the start of the successful fetch, with a 60-second stale window and no age
limit for fallback after failure. A read that asks for a fetch or refresh
arms the alarm immediately or keeps an earlier pending alarm. The experiment
has no waiting readers, so a read that needs the fetch result returns 202 even
when it holds a fallback for a failed fetch. If the policy allows the cached
value to be served immediately, a read can return it while a refresh runs.

On failure the object records the failed reply and backoff, keeps the value,
and logs `worker_experiment.stale_value_served` when the policy selects it
after a failure. The first retry is due 30 seconds after the failed fetch
started. A successful refresh replaces the value and clears the failure.
Alarms stop when the freshness crate considers the fixture idle. If SQLite
rejects a read-time write, the object logs the failure and keeps the new time
in memory on this object instance for the read and alarm. A later read
retries the write. If the object is evicted before the alarm fires and the
write is still pending, the alarm sees the older stored time.

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
state directory unless it explicitly reuses one. The checked reply schema
uses `fixture_cache_entry`; old local state should use a new persistence
directory for these checks. Use `loopback_dev` above
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
     SELECT last_read_ms, value_started_ms, value_payload,
            failure_count, backoff_until_ms, failure_payload
     FROM fixture_cache_entry;'
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

### Rejected read-time write

With a fresh disposable state directory, fill the value and identify its
database as above. While Wrangler is running, run this before the next alarm
is due. It ages the value past its stale window, marks the fixture idle, and
rejects writes that would update the read time.

```sh
sqlite3 "$db" <<'SQL'
UPDATE fixture_cache_entry
SET last_read_ms = (unixepoch() - 49 * 3600) * 1000,
    value_started_ms = (unixepoch() - 120) * 1000;
CREATE TRIGGER reject_read_time BEFORE UPDATE OF last_read_ms ON fixture_cache_entry
BEGIN
    SELECT RAISE(ABORT, 'fixture blocked read time');
END;
SQL
```

Request `/experiment/value`. Expect 202 while the alarm fetches, even though
SQLite holds a fallback. The Worker logs a `write_read_time` failure and
stores a reply with a newer `value_started_ms`. The row's `last_read_ms`
still shows the old time. Drop the trigger and repeat the GET to check that
the Worker persists the read time on retry.

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
then read `/experiment/value`. Expect 42 and a stale-serving event.
In `fixture_cache_entry`, the value reply and fetch start stay unchanged,
`failure_count` is 1, and `failure_payload` contains the 503 failure.
Restore the normal fixture before `backoff_until_ms`. After the recovery
alarm, the successful value has a new fetch start and the failure columns
are NULL. SQLite can confirm recovery even before the next GET.

In `stall` mode, the first request stalls before headers and the next stalls
after the first body byte. Both should log the typed `timeout` failure after
five seconds. With empty state the completed reply includes that failure.
The retry alarms run after the 30- and 60-second backoffs. The third
attempt should store 42 and clear the failure.

### Restart before the alarm

With a fresh state directory, request `/experiment/value` and poll until
it returns 42. Record `value_started_ms` from SQLite and stop Wrangler
before the next refresh, which is due five seconds after that fetch started.
Restart Wrangler with the same persistence directory. Query SQLite again
before making another HTTP request. The alarm should load the stored job and
store a reply with a newer `value_started_ms`. The first GET after the write
should return 42.

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

`mise run check` checks workspace formatting, documentation, dependency
boundaries and public API listings for host and WASM with default and all
features. It runs host Clippy, offline Rust tests, doctests, tool tests, and
checks the freshness crate's WASM builds without std.
`mise run check:wasm` builds, lints, documents and tests the shared crates
on WASM and compares their test lists with the host. The shared route tests
check fixture naming, stable JSON, status construction, body decoding,
failure classification, and the common event fields. On pull requests,
GitHub Actions builds and tests the native
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
