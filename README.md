# ccc-server-next

A Rust reimplementation of [ccc-server](https://github.com/frog-pond/ccc-server), the cautious-computing-context backend: it answers the CCC API by querying upstreams, caching their answers, and converting them to CCC's schema.

## Status

The server is being rebuilt on the `rearchitecture` branch, and this is that branch. The goal is to serve everything ccc-server's Cloudflare Worker serves, from the same code, both as a standalone docker image and as a Cloudflare Worker. [#424](https://github.com/frog-pond/ccc-server-next/issues/424) describes the effort and lists its parts.

The previous attempt's crates have been removed from this branch. They are still on `main`, where the old server can be built and run.

## Where to read

| Path | What it is |
|---|---|
| `AGENTS.md` | How the server is built and how to work in this repository. Written for coding agents, and the quickest orientation for people too. |
| `docs/reference/` | What the reference implementation serves and how it behaves, as of a recorded reference commit. |
| [Conversion fixtures](fixtures/ccc-server/README.md) | Reference inputs and expected outputs for offline Rust tests. |
| `docs/review/rubric.md` | The rules a change is reviewed against. |
| `docs/agent-hosts.md` | How to run the agents' procedures from T3 Code, Claude Code, the Codex CLI or by hand. |
| `docs/delegation-levels.md` | How much a coordinating agent may do on its own, and how the maintainer grants it. |
| `.claude/skills/` | Step-by-step procedures: working an issue, designing types, porting a route, reviewing a change, syncing with the reference, writing an issue, keeping the board in order and driving the whole effort. |

The reference implementation is expected to be checked out beside this repository, at `../ccc-server`. `tools/reference-path` prints where it was found.

## Environment

Tooling is managed with [mise](https://mise.jdx.dev).

```sh
brew install mise
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc
source ~/.zshrc
mise trust
```

```sh
rustup target add wasm32-unknown-unknown
mise install               # download the pinned tools
mise run setup:api         # install the pinned nightly and its Wasm target for API listings
cargo fetch --locked --target host-tuple --target wasm32-unknown-unknown
mise run check             # run the host and repository checks
mise run board             # the state of every issue under #424
mise run reference:drift   # what changed in the reference since the recorded commit
```

`mise install` and `cargo fetch --locked --target <target>` use the network
during setup. Fetch dependencies for each target you plan to test, as CI does.
After setup, run the tests without network access as CI does.

```sh
MISE_OFFLINE=true MISE_AUTO_INSTALL=false mise run test
```

After changing a library's public API, run `mise run update:api` and commit the
updated listings in [`docs/api/`](docs/api/). The listings cover workspace
libraries with both default and all features on the host and, for libraries
built for Wasm, on `wasm32-unknown-unknown`.

`mise run check` also compiles `trifecta-freshness` for Wasm without `std`,
with and without `jiff`.

CI runs the host workspace and the shared crates on stable and beta. To run the
shared checks on `wasm32-unknown-unknown` locally, make Node.js available and
run the following commands. Use `#[wasm_bindgen_test(unsupported = test)]`
for shared tests so both runners execute them. The Wasm check also compares the
test names reported by the host and Wasm runners. In each crate with shared
tests, add `wasm-bindgen-test = "=0.3.77"` to the unconditional
`[dev-dependencies]` table. It matches the Wasm runner pinned in
`.config/mise/config.toml`. Renovate updates the two pins together.

```sh
mise install github:wasm-bindgen/wasm-bindgen
cargo fetch --locked --target host-tuple --target wasm32-unknown-unknown
MISE_OFFLINE=true MISE_AUTO_INSTALL=false CARGO_NET_OFFLINE=true mise run check:wasm
```

## Native server

The native server accepts `GET /ping` on any host and returns plain-text
`pong`. The reference Worker restricts this route to Node-era hosts. The native
server accepts `localhost` so the container can check its own health. Other
methods, including `HEAD`, receive 405. An unmatched GET receives 404.
[#453](https://github.com/frog-pond/ccc-server-next/issues/453#issuecomment-7089629578)

Native `/ping` currently omits `Cache-Control` and `ETag`. [#449](https://github.com/frog-pond/ccc-server-next/issues/449) adds both.

The native upstream client identifies itself as `ccc-native/<crate version>`.
The reference Worker sends `ccc-server/2.0`. This difference identifies the
native build that sent a request. [#453](https://github.com/frog-pond/ccc-server-next/issues/453#issuecomment-7089629578)

Set `CCC_LISTEN_ADDR` to a comma-separated list of IP addresses with ports,
such as `0.0.0.0:8080,[::]:8080`. An unset or blank value selects
`0.0.0.0:8080,[::]:8080`. On hosts with IPv6 disabled, set
`CCC_LISTEN_ADDR=0.0.0.0:8080`. IPv6 listeners accept IPv6 connections only.
The native server binds every address before serving and drains active requests
for up to eight seconds on SIGTERM or SIGINT. A second signal ends the wait immediately.

The container runs `ccc-native healthcheck`. The command requests `/ping` on the
configured addresses, using loopback for wildcards. It exits successfully when
one listener returns HTTP 200 with the exact body `pong`. The default health
check uses loopback on port 8080.
The command requires fixed listener ports. It cannot discover the port assigned
to the server by the operating system when the configuration uses port 0.

```sh
docker build --tag ccc-native .
docker run --rm --publish 8080:8080 ccc-native
```
