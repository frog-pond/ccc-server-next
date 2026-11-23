# ccc-server-next

A Rust reimplementation of [frog-pond/ccc-server](https://github.com/frog-pond/ccc-server). The server answers the CCC API for the All About Olaf and Carleton apps: it asks other sites (BonApp, WordPress, calendars and others) for their data, caches their answers so those sites are not asked on every request, and converts the answers into the shapes the apps expect.

The repository is being rebuilt. Read this file before changing anything.

## Where things stand

- **Goal:** serve everything the reference's Cloudflare Worker serves, from the same code, both as a program in a Docker container and as a Cloudflare Worker.
- **Where the work happens:** the `rearchitecture` branch, not `main`. Issue #424 describes the effort and lists its parts.
- **Removed:** `ccc-server`, `ccc-server-routes`, `ccc-types`, `ccc-upstream-handlers` and `ccc-upstream-proxy`, the previous attempt. They still exist on `main`. Do not bring them back or copy how they were written.
- **Kept:** `e2e`, which compares this server's responses with a running reference.
- **Not built yet:** `trifecta` (#425 and its sub-issues describe it; the `trifecta` branch holds an unmerged draft, which is a starting point and not a precedent), the Worker build, cache backends, and tests that compare conversions with the reference's output. If a document or skill mentions one of these, check that it exists before relying on it.

## The reference implementation

- `tools/reference-path` prints where the reference is checked out. It is normally `../ccc-server` beside the main checkout, and the script also finds it from a git worktree. Set `CCC_REFERENCE_DIR` to override.
- The reference is read-only. Never edit it, build into it, or commit in it.
- `docs/reference/` describes what the reference serves and how it behaves, as of the commit in `docs/reference/REFERENCE_COMMIT`. `tools/reference-drift` reports what has changed in the reference since then.
- When those documents and the reference's code disagree, the code at that commit is right. Fix the document.

## How the server is built

- **What it serves.** Everything in `docs/reference/routes.toml` except where an issue defers it: `/previews` and `POST /sentry/telegram` are deferred (#434), and the job and organisation boards are designed separately (#435). That is the routes the reference's Worker serves, under `/edu.stolaf/` and `/edu.carleton/`, plus the older `/v1` addresses that released apps still call. The Node server's routes that take an arbitrary URL are not carried over.
- **Two builds from one set of libraries.** A native program for Docker and a Cloudflare Worker compiled to WebAssembly. Each has its own small crate containing the entry point, and which one you build decides the target. There is no Cargo feature that switches between them: Cargo turns on every feature at once in a whole-workspace build, so two features that exclude each other cannot be linted or tested together.
- **Each outside resource is described once.** A description has a name that never changes, the parameters that pick out one resource (one café, one feed), and how long its value stays fresh. Parameters and values can be serialized.
- **A background refresh is a name plus parameters.** It is never a closure or a spawned task, because a Cloudflare Durable Object wakes up with nothing but what it stored.
- **One place decides whether a cached value is still fresh.** That code does no network or storage access itself. The Docker build and the Worker build each wrap it and do the access it asks for, so the rules are written and tested once.
- **Libraries are handed what they need.** They do not read the clock, environment variables or global state. The crate with the entry point builds those and passes them in. On Cloudflare, settings and secrets arrive with each request, so setup has to be cheap and repeatable.
- **Failures are returned, not thrown, across a Durable Object call.** Cloudflare reports every exception that crosses that call as unhandled, and another site being down is routine.
- **`trifecta` is internal.** It is the crate that runs "fetch, adapt, convert" for one resource. A change to it names the route or build that needs it.

## Rules

Matching the reference comes first: accept what it accepts and send what it sends (rule P0). Within that, the point of writing this server in Rust is its type system and its performance. That is the maintainer's primary objective for the project, and the rules below serve it. When a type could be made more exact, make it more exact.

Full wording and reasons are in `docs/review/rubric.md`. Reviews check against that file.

- **Separate types for what a site sends and what the apps receive.** The first mirror the site's data and derive `Deserialize` but not `Serialize`. Convert with `TryFrom` and a typed error.
- **Make invalid states unrepresentable.** Newtypes for identifiers from an open set, enums for closed sets, `Option` only where the site really omits the field. No `serde_json::Value` in a schema, except where the reference passes a value through untouched (rule T6).
- **A struct or an enum, whichever fits the data.** Fields that exist together are a struct. Exactly one of several cases is an enum. Several `Option` fields of which one is expected to be set, or a flag beside an `Option`, should be an enum.
- **Traits are deliberate.** A trait describes a capability that more than one type provides, such as a cache backend. Do not add one for a single implementation.
- **Document every public item in detail.** Types, fields, variants, functions and modules: what the item represents, any rule about its contents, when a function fails, and for a site's type where its shape comes from.
- **Parse each response once**, and do not copy a body or a collection without a reason.
- **No panics outside tests.** No `unwrap`, `expect`, indexing that can fail, or `todo!()` in merged code.
- **Shared libraries build for `wasm32-unknown-unknown`** and do not depend on `tokio`, `reqwest`, `axum` or `worker`.
- **Typed errors.** Each library crate has its own error enum implementing `std::error::Error`. No `String` errors, no boxed catch-alls.
- **Internal failures are recorded.** The cache being unavailable, a refresh failing or a stale value being served after a failed refresh emits a structured event with a stable name. Never swallow one.
- **Conversions do no I/O** and read no clock or environment, so both builds share them and one set of tests covers them.
- **Tests never contact a live site.** Use the reference's fixtures. Do not `curl` a site to see what it returns; several already rate-limit or challenge the reference. Ask the maintainer if a new capture is needed.
- **When a sandbox blocks a command, ask.** Request approval to run that one command outside the sandbox, through your host's approval mechanism. Do not work around the sandbox, and do not skip the step without saying so.
- **Do not trust memory for Cloudflare or `worker` crate APIs.** Read the crate's source under `~/.cargo/registry` or the current documentation.

## Where things are written down

| What | Where | Notes |
|---|---|---|
| Why a change is being made | The GitHub issue for it | Written for people, in plain terms. |
| How the server works | `docs/` and this file | Facts about how things are, not history. |
| What the reference serves | `docs/reference/routes.toml` | Facts about the reference only. Nothing about progress here. |
| Who is working on what | GitHub issues, one per piece of work | The pull request links the issue. |
| Whether a route is finished | Worked out from merged pull requests | Never keep a list of finished routes in a file. |
| What is in progress | The project board "@rye's Rearchitecting Push" | `tools/board` reads it. |
| A pull request's review | The "Review" section of its description | Edited in place for each round. No comment per review. |
| Grants of autonomy, driver handoffs, and the driver's questions for the maintainer | The driver's session (a T3 thread, or another host's session) | Ask questions about an issue's design in that issue's session. The handoff identifies each issue's worker session. |
| Worker reports and issue design discussions | The issue's own top-level session | The report is the worker's final message. The maintainer can open the session and talk to the worker directly. `docs/agent-hosts.md` says how on each host. |
| Discussion and exploration | The conversation | Anything a person reading the issue later needs goes in a row above. |

The design of `trifecta` is described in #425 and its sub-issues. The maintainer also keeps working notes locally. They are not in the repository, so do not assume they are present, and do not guess their contents.

Comments on issues and pull requests are kept for what a person reading them later needs, such as a question only the maintainer can answer. Progress, handoffs and review rounds do not go in comments. When a comment is superseded, minimize it as outdated.

Issues and documents are written for a person who has not followed the work: say what happens and why in ordinary words, and do not invent names for things.

## How work flows

Work is tracked in GitHub issues under #424. `tools/board` shows their state and which are ready to start.

1. To work one issue, follow the `work-issue` skill. To coordinate the whole effort, follow `drive-rearchitecture`. Claude drives; Codex works issues and reviews. The skills work on any host that can run an agent in a git worktree; `docs/agent-hosts.md` maps each step to T3 Code, Claude Code, the Codex CLI and doing it by hand. A driver works at the level of autonomy the maintainer grants, described in `docs/delegation-levels.md`; with no grant it is at level 1.
2. One issue per branch and pull request, in its own worktree. Branches are named `rearch/<issue>-<name>`. A change to the tools, skills or agent instructions that the maintainer asks for directly needs no issue: its branch is `rearch/<topic>`, with no number, and `tools/board` does not list it. Reviewers do not flag such a branch for having no issue.
3. Branch from the base `tools/board` prints for the issue: `rearchitecture`, or the branch of the pull request the issue is waiting on while that is unmerged. Foundation work therefore forms a stack of pull requests, and routes are independent of each other.
4. Nothing pushed is rewritten: no rebase and no force-push. When a branch you built on changes, merge it into yours.
5. A pull request is opened as a draft. Types and signatures are reviewed in the draft before the bodies are written, and the bodies are added to the same pull request (`design-types`, `port-route`). A draft is never merged.
6. `mise run check` passes before review. Review follows the `arch-review` skill, which decides how thorough the review is from what the change touches.
7. You may push `rearchitecture` and `rearch/*` branches and open pull requests between them without asking. Never push to `main`, open a pull request against it, or merge into it. `main` is also protected on GitHub: it accepts only pull requests with passing checks. Merging a pull request is the maintainer's, unless they have granted level 5 for that work (`docs/delegation-levels.md`).

## Commands

```sh
tools/board         # every issue under #424, what blocks it, and the open pull requests
tools/board maintain [--apply]   # the board moves the rules call for (see the board-maintenance skill)
mise run check      # host lint, formatting, tests, doctests, docs, dependency bans, API listings and offline tool checks
mise run check:wasm # build, lint, check docs and run tests for the shared crates on Wasm
mise run clippy
mise run fmt
mise run test
mise run test:tools # offline checks of board rules and workspace structure
tools/reference-path    # where the reference is checked out
tools/reference-drift   # what changed in the reference since the recorded commit
```

`mise run e2e` contacts live services. Do not run it without being asked.

When an entry point needs a disallowed method, use
`#[expect(clippy::disallowed_methods, reason = "...")]` at the call site and
explain why that call belongs there.
