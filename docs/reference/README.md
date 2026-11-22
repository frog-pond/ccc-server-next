# Reference extracts

Facts extracted from the reference implementation, [frog-pond/ccc-server](https://github.com/frog-pond/ccc-server), at the commit named in `REFERENCE_COMMIT`. They exist so that nobody has to rediscover cross-cutting behaviour from a dozen files each time. They do not replace reading the reference when porting a route.

| File | What it holds |
|---|---|
| `REFERENCE_COMMIT` | The reference commit these extracts describe. |
| `routes.toml` | Every route the Worker serves, per campus: its sources, upstream hosts, shape functions, failure contract, tests and fixtures. One row is one unit of porting work. |
| `cache-semantics.md` | The exact behaviour of the Worker's source cache: freshness, background refresh, backoff, epochs, conditional requests and the client-facing HTTP layer. This is the specification for the freshness crate. |
| `upstreams.md` | Every upstream service: hosts, request shape, authentication, quirks, and an index of the shape modules. |

## Rules

- These files describe the reference only. They never record whether something has been ported here. That is derived from issues and merged pull requests.
- Every claim cites `file:line` relative to the reference root. A claim marked `UNVERIFIED` has not been confirmed against the code.
- When an extract and the reference code at the pinned commit disagree, the code wins. Fix the extract in the same pull request that found the difference.
- `tools/reference-drift` shows what changed in the reference since the pinned commit. The `sync-reference` skill describes how to bring the extracts up to date and move the pin.
