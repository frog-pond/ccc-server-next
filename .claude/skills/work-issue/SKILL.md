---
name: work-issue
description: Take one GitHub issue in this repository from start to an open pull request on the rearchitecture stack. Use when asked to work on, pick up, implement or continue an issue here, or when handed an issue number by a driver.
---

# Work an issue

One issue, one branch, one pull request. The issue says what and why. `AGENTS.md` says how the server is built.

## 1. Before starting

1. Read the issue, its parent, and the issues it is blocked by: `tools/board` shows them. If one it is blocked by is still open and its branch is not on the stack, stop and say so.
2. Read `AGENTS.md`. For a route, follow `port-route`. For any new or changed type, follow `design-types`.
3. Set the issue's status to "In progress" on the project board, and link the issue to your thread if the host offers that.

## 2. Branch

- Work in your own git worktree, never in the main checkout.
- Name the branch `rearch/<issue number>-<short-name>`.
- Branch from the work it builds on:
  - foundation work branches from the top of the stack, which `tools/board` prints;
  - a route branches from `rearchitecture`, or from the top of the stack if what it needs has not merged yet.
- Never rewrite a branch that has been pushed. If the branch you built on gains commits, merge it into yours. Do not rebase and do not force-push.

## 3. Do the work

- Do what the issue's "Changes" say, and nothing it does not ask for. If the work needs a decision the issue does not make, stop and ask in a comment on the issue.
- Check each "Done when" item yourself and keep the evidence: a command and its output, or a test name.
- `mise run check` passes. A crate meant for both builds also passes `cargo check --target wasm32-unknown-unknown -p <crate>`.
- Commit in small steps. Subjects are imperative with the area first, as the history has them: `types: Add the WordPress post schema`.

## 4. Open the pull request

- Push the branch and open a pull request whose base is the branch you branched from. Never open one against `main`.
- Fill in the pull request template. Say `Part of #N` for the issue. A merge into `rearchitecture` does not close an issue by itself, so do not rely on `Closes`.
- Set the issue's status to "In review".
- Run the `arch-review` skill and deal with what it finds.
- Do not merge it yourself.

## 5. If you stop before finishing

Leave a comment on the issue, because nobody can read your conversation:

```
State: <what is done, what is not>
Branch: <name>, last commit <sha>
Next: <the next concrete step>
Questions: <anything that needs the maintainer>
```

## An experiment

Some issues ask for an experiment first, such as #429. Do it on a branch named `rearch/<issue>-experiment` that is never merged. The result is a comment on the issue saying what was tried, what happened and what it means for each open question, with the branch name for anyone who wants the code.

## Never

- Push to `main`, or open a pull request against it.
- Edit, build into or commit in the reference checkout.
- Contact a live upstream.
- Mark something done that you have not checked.
