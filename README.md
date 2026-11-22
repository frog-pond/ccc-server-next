# ccc-server-next

A Rust reimplementation of [ccc-server](https://github.com/frog-pond/ccc-server), the cautious-computing-context backend.

## Status

The server is being rebuilt on the `rearchitecture` branch, and this is that branch. [#424](https://github.com/frog-pond/ccc-server-next/issues/424) describes the effort and lists its parts.

The previous attempt's crates have been removed from this branch, so there is no server to run here yet. They are still on `main`, where the old server can be built and run.

## Environment

Tooling is managed with [mise](https://mise.jdx.dev).

```sh
brew install mise
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc
source ~/.zshrc
mise trust
```

```sh
mise run check   # clippy, fmt-check and test
```
