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
rustup toolchain install nightly-2026-11-22 --profile minimal # public API listings
mise run check             # lint, test, check docs, dependencies and API listings
mise run board             # the state of every issue under #424
mise run reference:drift   # what changed in the reference since the recorded commit
```

After changing a library's public API, run `mise run update:api` and commit the
updated listings in [`docs/api/`](docs/api/). The listings cover workspace
libraries with all features enabled on the host.

CI runs the host workspace and the shared crates on stable and beta. To run the
shared tests on `wasm32-unknown-unknown` locally, make Node.js available and run
the following commands. Use `#[wasm_bindgen_test::wasm_bindgen_test]` for tests
that should execute in Wasm.

```sh
rustup target add wasm32-unknown-unknown
mise install github:wasm-bindgen/wasm-bindgen
mise run test -- --target wasm32-unknown-unknown --exclude ccc-native --exclude e2e
```

## Native server

The native server accepts `GET /ping` on any host and returns plain-text
`pong`. The reference Worker restricts this route to Node-era hosts. The native
server accepts `localhost` so the container can check its own health. Other
methods, including `HEAD`, receive 405. An unmatched GET receives 404.
[#453](https://github.com/frog-pond/ccc-server-next/issues/453)

The shared response layer will supply `Cache-Control: public, max-age=600`
and the ETag. [#449](https://github.com/frog-pond/ccc-server-next/issues/449)

Set `CCC_LISTEN_ADDR` to a comma-separated list of IP addresses with ports,
such as `0.0.0.0:8080,[::]:8080`. An unset or blank value selects
`0.0.0.0:80,[::]:80`. IPv6 listeners accept IPv6 connections only. The native
server binds every address before serving and drains active requests on SIGTERM.

The container's health check targets `localhost:80`. Keep port 80 available
inside the container when changing its listen addresses.

```sh
docker build --tag ccc-native .
docker run --rm --publish 8080:80 ccc-native
```
