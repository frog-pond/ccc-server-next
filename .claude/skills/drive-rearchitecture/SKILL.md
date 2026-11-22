---
name: drive-rearchitecture
description: Run the rearchitecture as a loop, choosing issues that are ready, handing each to a worker, getting pull requests reviewed and keeping the stack and the board in order. Use when asked to drive, run, continue or make progress on the rearchitecture as a whole, not on one issue.
---

# Drive the rearchitecture

You are coordinating, not implementing. This skill is written for Claude as the driver, the maintainer's choice; Codex works issues and reviews. Each issue is worked by a separate worker in its own worktree, following `work-issue`. Your context is for the state of the whole effort, so keep implementation detail out of it.

The parent issue is #424. The board is the project "@rye's Rearchitecting Push".

## Starting a session

1. Run from a worktree, not the main checkout, which holds the maintainer's own work. `git fetch origin`, then use a worktree of `origin/rearchitecture`. If `tools/board` is missing there because the scaffolding has not merged yet, use the branch of the open pull request based on `rearchitecture` that contains it (`gh pr list --base rearchitecture`).
2. Name this session `Driver: <the work>` if the host lets you. Find the previous driver session and read its last message: it says what was left in flight and what is waiting on the maintainer. `docs/agent-hosts.md` says how on each host; if yours cannot, ask the maintainer for it.
3. Reading needs no grant: the board, issues, pull requests, the reference and earlier sessions. Find your level in `docs/delegation-levels.md`. If the maintainer granted one in this session, or pointed you to a session whose grant still covers the work, use it. Otherwise run `tools/board` and choose the work. Then ask the two questions in `docs/delegation-levels.md` together, "What level can I run at?" and "Where should I stop?", each with options written for that work, the recommended one first and its reason in one line. State the grant back in one line: the level, the issues it covers and the end point. With no answer about the level, you are at level 1; with none about the end point, stop after one round.
4. Before every action, check it is within your level and not on the list escalated at every level. If it is not, bring it to the maintainer as options with a recommendation.
5. Hand each issue to its own worker in its own worktree, using whatever your host offers for starting an agent with its own checkout (`docs/agent-hosts.md`). For the second reviewer of a thorough review, use a model from another vendor where one is available.

## Ending a session

Before you stop, end with this as your last message in the session. Do not post it on an issue.

```
State: <what moved this session: pull requests opened, reviewed, ready to merge, merged>
Level: <the grant this session ran under, and whether it reached its end point>
In flight: <each worker still running, its issue and its level>
Next: <what the next session should do first>
Waiting on the maintainer: <one line per item, with the issue or pull request number>
```

At level 4 or 5, add "Decisions I made": each question you answered or finding you ruled on, in one line with its issue or pull request.

Then tell the maintainer the "Waiting on the maintainer" list.

## Each time round

1. **Look.** Follow `board-maintenance` first, so the board matches the issues and pull requests. Then run `tools/board`. It lists every issue under #424 with its status, priority, size, estimate, what blocks it, its pull request and whether it is ready. Run `tools/reference-drift` too. If the reference has moved, tell the maintainer and do not start new route work; syncing needs the maintainer (`sync-reference`).
2. **Tend what is in flight first.** For each open pull request on the stack:
   - checks failing: send it back to a worker with the failure, using the follow-up request below;
   - not yet reviewed at its current head: run `arch-review`. Workers do not review their own work. A pull request's review is the "Review" section of its description, naming the commit it covers; GitHub's review decision is separate and is the maintainer's;
   - findings outstanding: send them to a worker;
   - a draft: leave it, unless its types are waiting for review;
   - reviewed at its current head and passing: it is ready to merge (see "Merging").
3. **Start what is ready.** An issue is ready when `tools/board` marks it (→) and prints a base for it. The board's help text is the only definition of ready. Start at most three at once, and never two that touch the same crate's public types. Among ready issues, take higher Priority first (Urgent, then High, then Medium, then Low), and within the same priority, what unblocks the most other issues. An issue with no Priority comes after Low; list it for the maintainer, as `board-maintenance` says.
4. **Hand over.** Give each worker the request below. A worker for a different issue is a new worker, not a follow-up message to an old one.
5. **Record.** Set statuses on the board, and follow `board-maintenance` again after pull requests open or merge. If you learned something later workers need, put it in the documentation or in your requests to them. Something that changes what an issue asks goes to the maintainer.
6. **Stop** at your grant's end point, when nothing is ready and nothing in flight can move, or when the next step needs the maintainer. Say exactly what is waiting on the maintainer, one line per item, with the issue number.

## The requests to a worker

```
Work issue #<N> in frog-pond/ccc-server-next, following the work-issue skill.
Branch from <the base the board prints>. Open the pull request against that branch.
Report: the pull request URL, the head commit, what you checked for each
"Done when" item, and any question for the maintainer.
```

To send a pull request back:

```
Continue issue #<N> in frog-pond/ccc-server-next on branch <branch>, following the
work-issue skill. Pull request #<P> needs: <the failing check, or the findings
to fix, each with file:line>. Report the new head commit and what you changed.
```

Add nothing about how to do the work. If the issue is not enough to work from, fix the issue.

## The stack

- Foundation work is a stack of pull requests: each is based on the branch before it, and the bottom one is based on `rearchitecture`.
- Routes are not stacked on each other. Each branches from the base the board prints for it.
- Nothing pushed is rewritten. When a lower branch gains commits, you merge it into each branch above it, bottom to top, and push, or send a worker to. No rebase, no force-push.
- Merge pull requests with a merge commit, not a squash, so the branches above already contain the merged commits. The repository deletes a branch when its pull request merges, and GitHub then points the pull requests that were based on it at `rearchitecture`. Check that it did.

## Merging

- Merge from the bottom of the stack upward, only when checks pass and the review is current: its "Review" section names the commit reviewed and accounts for every commit after it (see `arch-review`). A draft is never merged.
- A change that got the thorough review also needs the maintainer's approval on GitHub before it merges. The maintainer reviews types and structure in person.
- Below level 5, do not merge: queue what is ready to merge and tell the maintainer. At level 5, the maintainer's grant is permission to merge what is ready within its scope; `AGENTS.md`'s "merging asks the maintainer" is met by the grant. The host may still ask the maintainer to confirm a merge; that is the host's check and does not change your level.
- After a merge, close the issue if every "Done when" item is met, and set it to "Done". Add the pull request's number to the parent issue where it lists the change.

## Splitting an issue

Some issues are too large for one pull request, such as #434. Split along the lines the issue names (one sub-issue per upstream). A sub-issue that only restates part of an approved issue may be filed directly with the short form in `write-issue`. An issue that makes a new argument or a new decision is drafted on the board for the maintainer first.

## When something needs the maintainer

Do not guess, and do not let it stop unrelated work. Ask in this session, with the options you see and a recommendation, leave the issue's status as it is, move on to other ready work, and list the question again when you stop. Questions from workers come to you in their reports; answer what your level allows and bring the rest here.

What goes to the maintainer depends on your level, and the list in `docs/delegation-levels.md` goes to them at every level.

## Never

- Push to `main`, open a pull request against it, or merge anything into it.
- Work an issue yourself to save time. It costs the context you need for coordinating.
- Mark an issue done because its pull request merged. Check its "Done when".
