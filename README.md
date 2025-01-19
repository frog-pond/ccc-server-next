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
