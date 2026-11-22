---
name: drive-rearchitecture
description: Run the rearchitecture as a loop, choosing issues that are ready, handing each to a worker, getting pull requests reviewed and keeping the stack and the board in order. Use when asked to drive, run, continue or make progress on the rearchitecture as a whole, not on one issue.
---

# Drive the rearchitecture

You are coordinating, not implementing. Each issue is worked by a separate worker in its own worktree, following `work-issue`. Your context is for the state of the whole effort, so keep implementation detail out of it.

The parent issue is #424. The board is the project "@rye's Rearchitecting Push".

## Each time round

1. **Look.** Run `tools/board`. It lists every issue under #424 with its status, size, what blocks it, its pull request and whether it is ready. Run `tools/reference-drift` too; if the reference has moved, follow `sync-reference` before starting route work.
2. **Tend what is in flight first.** For each open pull request on the stack:
   - checks failing: send it back to a worker with the failure;
   - not yet reviewed: run `arch-review`;
   - findings outstanding: send them to a worker;
   - reviewed and passing: it is ready to merge (see "Merging").
3. **Start what is ready.** An issue is ready when everything that blocks it is closed or has a pull request on the stack. Start at most three at once, and never two that touch the same crate's public types. Prefer what unblocks the most other issues.
4. **Hand over.** Give each worker the request below. A worker for a different issue is a new worker, not a follow-up message to an old one.
5. **Record.** Set statuses on the board. If you learned something another worker needs, put it in a comment on the issue or in the documentation, not in your conversation.
6. **Stop** when nothing is ready and nothing in flight can move, or when the next step needs the maintainer. Say exactly what is waiting on him, one line per item, with the issue number.

## The request to a worker

```
Work issue #<N> in frog-pond/ccc-server-next, following the work-issue skill.
Branch from <base branch>. Open the pull request against <base branch>.
Report: the pull request URL, the head commit, what you checked for each
"Done when" item, and any question for the maintainer.
```

Add nothing about how to do the work. If the issue is not enough to work from, fix the issue.

## The stack

- Foundation work is a stack of pull requests: each is based on the branch before it, and the bottom one is based on `rearchitecture`.
- Routes are not stacked on each other. Each branches from `rearchitecture`, or from the top of the stack while what it needs is unmerged.
- Nothing pushed is rewritten. When a lower branch gains commits, merge it into each branch above it, bottom to top, and push. No rebase, no force-push.
- Merge pull requests with a merge commit, not a squash, so the branches above already contain the merged commits. GitHub then points the next pull request at `rearchitecture` by itself.

## Merging

- Merge from the bottom of the stack upward, only when checks pass and the review is recorded in the pull request.
- A change that got the thorough review also needs the maintainer's approval on GitHub before it merges. He reviews types and structure himself.
- Merging asks the maintainer for confirmation each time. Until he changes that, queue what is ready to merge and tell him.
- After a merge, close the issue if every "Done when" item is met, and set it to "Done". Add the pull request's number to the parent issue where it lists the change.

## Splitting an issue

Some issues are too large for one pull request, such as #434. Split along the lines the issue names (one sub-issue per upstream). A sub-issue that only restates part of an approved issue may be filed directly with the short form in `write-issue`. An issue that makes a new argument or a new decision is drafted on the board for the maintainer first.

## When something needs the maintainer

Do not guess, and do not let it stop unrelated work. Comment on the issue with the question and the options you see, leave the issue's status as it is, move on to other ready work, and list the question when you stop.

These always go to him:

- a difference from the reference that seems deliberate or necessary;
- two reviewers disagreeing about structure;
- anything that changes "How the server is built" in `AGENTS.md`;
- a type design for the first route of an upstream.

## Never

- Push to `main`, open a pull request against it, or merge anything into it.
- Work an issue yourself to save time. It costs the context you need for coordinating.
- Mark an issue done because its pull request merged. Check its "Done when".
