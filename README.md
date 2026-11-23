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
rustup toolchain install nightly-2026-11-22 --profile minimal # public API listings
cargo fetch --locked       # cache dependencies before running tests offline
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
mise run test -- --target wasm32-unknown-unknown --exclude e2e
```
