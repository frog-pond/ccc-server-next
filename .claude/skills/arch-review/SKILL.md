---
name: arch-review
description: Review a change in this repository against its architecture rubric, choosing a light single-reviewer pass or a heavy two-model review from what the diff touches. Use on a pull request here before it is marked ready or merged, or when asked to review a branch, diff or pull request in this repository.
---

# Review a change

The rules are in `docs/review/rubric.md`, and how the server is meant to be built is in `AGENTS.md`. A reviewer gets those and the diff. A reviewer does not get the author's conversation or the author's account of why the change is fine: someone who is handed the author's reasoning tends to repeat its mistakes.

## 1. Decide how thorough the review is

Run `git diff --stat rearchitecture...HEAD` (or against the pull request's base). The change gets the **thorough** review if it does any of these:

- changes something public in `trifecta/`: a trait, a public type or a signature;
- adds a crate, adds an outside dependency, or changes which workspace crate depends on which;
- touches a cache backend, the freshness rules, or code specific to the Docker build or the Worker build;
- changes an error enum used by more than one crate, or the name of a recorded event or one of its fields;
- touches `AGENTS.md` or `docs/review/rubric.md`;
- adds the types for the first route that reads from a given upstream.

Otherwise it gets the **quick** review. If you are unsure, it is thorough.

## 2. Quick review

One reviewer starting from nothing, on a different model from the author where one is available. Give it the request below.

## 3. Thorough review

Two reviewers working separately, one Claude and one Codex:

- Claude: the `arch-reviewer` subagent, or another Claude started fresh.
- Codex: through whatever the host offers for handing work to Codex, or `codex review` from a shell.

Give both the same request. Neither sees what the other found.

## The request

```
Review the change on branch <branch> against <base> in <repository path>.
Read AGENTS.md and docs/review/rubric.md first.
Check the change against each rule that applies. For each finding give:
the rule, file:line, the evidence, the consequence, and a specific change.
Check each finding in the code before reporting it. Do not report formatting
or anything clippy catches. If the change goes against "How the server is
built" in AGENTS.md, say which point. If you could not check something, say
so instead of guessing.
End with one line: approve, approve with changes, or reject.
```

For a route, add its `id` from `docs/reference/routes.toml` and ask the reviewer to compare the types with the reference files that entry names.

## 4. Act on the findings

1. Check each finding in the code yourself. Drop any you cannot reproduce, and say why.
2. Fix the ones that hold.
3. Where the two reviewers disagree about structure, or you disagree with a finding that holds, take it to the maintainer. The author does not settle it alone.
4. Write in the pull request: which kind of review it was, which models reviewed, the commit they reviewed, each finding and what was done about it.

A review covers the commit it read. Types reviewed in a draft are reviewed again, with their bodies, before the pull request is marked ready.

Review again after fixing only if a fix changed a public type, a dependency between crates, or how the server is built. A second review is a new reviewer starting from nothing, not a follow-up message to the first.
