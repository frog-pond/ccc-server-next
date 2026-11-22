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
| `.claude/skills/` | Step-by-step procedures: working an issue, designing types, porting a route, reviewing a change, syncing with the reference, writing an issue and driving the whole effort. |

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
mise run check             # clippy, fmt-check and test
mise run board             # the state of every issue under #424
mise run reference:drift   # what changed in the reference since the recorded commit
```
