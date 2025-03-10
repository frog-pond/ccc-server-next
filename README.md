# ccc-server-next

next generation of cautious-computing-context

## Environment

Config is managed with [mise](https://mise.jdx.dev)

Install

```sh
brew install mise
echo 'eval "$(mise activate zsh)"' >> ~/.zshrc
source ~/.zshrc
mise trust
```

Build and run

```sh
mise use -g rust
mise run build
mise run start
```

**`Bonappetit`**: You'll need to have authorization setup to get bonappetit requests working. Copy `.env.sample` to `.env` and set the token to get this working.

### Logging and Tracing

You can configure the logging level using the `RUST_LOG` environment variable:

```sh
# logging level: error, warn, info, debug, and trace
RUST_LOG=debug mise run start
```

The server also supports different tracing formats which can be set using the `-t` or `--tracing` flag:

```sh
# tracing formats: default, debug, json, pretty
mise run start -- -t json
```

Combine log level and tracing for different levels of output and formats:

```sh
RUST_LOG=trace mise run start -- -t pretty
```
