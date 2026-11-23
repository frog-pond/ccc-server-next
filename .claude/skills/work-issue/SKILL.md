---
name: work-issue
description: Take one GitHub issue in this repository from start to an open pull request on the rearchitecture stack. Use when asked to work on, pick up, implement or continue an issue here, or when handed an issue number by a driver.
---

# Work an issue

One issue, one session, one branch, one pull request. The issue says what and why. `AGENTS.md` says how the server is built.

## 1. Before starting

Read the issue and `AGENTS.md` on every start. For a route, follow `port-route`. For any new or changed type, follow `design-types`.

If you are returning to an issue, use its existing worktree. If the issue has a branch, check that the worktree is on that branch and check the pull request if one has opened. Then follow sections 3 and 4 for the work in the follow-up request. If you stopped before creating the branch, resume the first-start steps below.

1. Run `tools/board`. If it does not mark the issue ready (→) and print `[start from …]` for it, stop and report the reason it gives. If the sandbox stops it from reaching GitHub, ask for approval to run it outside the sandbox; the same goes later for `git push`, `gh` and `tools/board status`. Readiness is defined in one place, the board's own help text, and nowhere else.
2. Read the issue's parent and the issues it is blocked by.
3. Create your branch (section 2), then set the issue's status to "In progress" (`tools/board status <N> "In progress"`), and link the issue to your session if the host offers that.

## 2. Branch

- Work in your own git worktree, never in the main checkout. If the host gave you a fresh worktree, use it; otherwise `git worktree add ../ccc-server-next-<N> origin/<base>`. If the issue already has a branch, use that branch in the issue's worktree.
- In a new worktree, `mise trust` may be needed once. The Worker build needs `rustup target add wasm32-unknown-unknown`.
- Name the branch `rearch/<issue number>-<short-name>`, where the short name is two to four words of the issue's title, lowercase and joined by hyphens (`rearch/428-check-structure-in-ci`). `tools/board` only recognizes branches named this way.
- If the branch needs to be created, use the base `tools/board` prints for the issue (`[start from …]`), after `git fetch origin`: `git switch -c rearch/<N>-<name> origin/<base>`. If the board prints no base, the issue is not ready: stop and say why.

## 3. Do the work

- Never rewrite a branch that has been pushed. Fetch the base when you return to a branch. If the branch you built on has gained commits, merge it into yours. Do not rebase and do not force-push.
- Do what the issue's "Changes" say, and nothing it does not ask for. If the work needs a decision the issue does not make, stop and put the question in your report.
- Check each "Done when" item yourself and keep the evidence: a command and its output, or a test name.
- `mise run check` passes. For changes to a crate meant for both builds, also run `mise run check:wasm`. That task needs the Wasm Rust target, Node.js and the pinned `wasm-bindgen` runner. See the README for setup.
- Commit in small steps, with short imperative subjects: `Add the WordPress post schema`. Say `Part of #<N>` in the body.

## 4. Open or update the pull request

- Push the branch. If it already has a pull request, update that pull request. Otherwise, open a draft against the branch you branched from. Never open one against `main`.
- For a new pull request, fill in the template. Say `Part of #N` for the issue. A merge into `rearchitecture` does not close an issue by itself, so do not rely on `Closes`.
- If the work adds or changes types, push the types and signatures first and get them reviewed in the draft before writing the bodies (`design-types`). Add the bodies to the same pull request.
- When the work is complete, mark the pull request ready for review, set the issue's status to "In review" (`tools/board status <N> "In review"`), and report in the issue's session. The driver arranges the review, because a worker cannot start other agents. When findings come back to you, fix them and say in your report what changed since the reviewed commit.
- Do not merge it yourself.

## 5. If you stop before finishing

End with this report in the issue's session. Whoever started the work reads it there. Put any design question for the maintainer in that session too.

```
State: <what is done, what is not>
Branch: <name and last commit, or "not created yet">
Next: <the next concrete step>
Questions: <anything that needs the maintainer>
```

## An experiment

Some issues ask for an experiment first, such as #429. Do it on a branch named `rearch/<issue>-experiment` that is never merged. The result is your report: what was tried, what happened and what it means for each open question, with the branch name for anyone who wants the code. The driver takes it to the maintainer, and what it settles is written into the issue or the documentation.

## Never

- Push to `main`, or open a pull request against it.
- Edit, build into or commit in the reference checkout.
- Contact a live upstream.
- Mark something done that you have not checked.
