---
name: board-maintenance
description: Bring the project board "@rye's Rearchitecting Push" in line with the issues and pull requests, moving issues that are no longer blocked to Ready and making the other moves the rules call for. Use when asked to tidy, maintain, groom or update the board, after pull requests open or merge, or at the start of a driving session.
---

# Maintain the board

The board shows each issue under #424 with a Status (Backlog, Ready, In progress, In review, Done), a Size, an Iteration and its assignees. Most of that follows from facts GitHub already has: which issues are closed, which block which, and which pull requests are open or merged. `tools/board maintain` works out the moves those facts call for. This skill is about running it and dealing with what it cannot decide.

## Steps

1. Run `tools/board maintain`. It is a dry run: it lists the moves it would make, then what needs a person's judgment.
2. Read the moves. If one looks wrong, do not apply anything; find out why. The rules are in `tools/board --help`, and a wrong move usually means an issue's blocked-by links or a branch name is wrong.
3. Run `tools/board maintain --apply`. It makes the same moves and prints them again.
4. Deal with each item under "Needs a person's judgment" (below).
5. Run `tools/board` and check the result.

## The moves it makes

- **Ready**: an open issue that has not been started and that `tools/board` marks ready. An issue marked Ready that is blocked again goes back to **Backlog**.
- **In progress** while the issue's own pull request is a draft, **In review** once it is not.
- **Done** once the issue is closed.
- A parent issue (one with sub-issues) is **In progress** once any sub-issue has started or finished, and **Backlog** before that. It is never Ready: its sub-issues are the work.
- An issue under #424 that is missing from the board is added.
- An open issue with nobody assigned is assigned to the maintainer.
- An issue that becomes Ready or starts, and has no iteration, goes into the iteration that covers today.

## What it leaves to you, and what to do

| It reports | What to do |
|---|---|
| A pull request merged, but its issue is open | Check the issue's "Done when". If every item is met, close the issue; the next run marks it Done. If not, say what is missing in your report. |
| Every sub-issue is closed, but the parent is open | The same, for the parent. |
| In progress or In review, with no branch and no pull request | Someone may have stopped without pushing. A driver checks its own workers; otherwise ask the maintainer. Do not reset the status. |
| In review, with a branch but no open pull request | The pull request was closed or never opened. Find out which before moving the issue. |
| Open, but marked Done | It was reopened, or closed by mistake. Ask. |
| No size | Sizes are the maintainer's estimate. List the issue for him; do not guess. |
| On the board but not under #424 | Either link it as a sub-issue of the right issue, or it belongs to other work. Ask. |
| A draft item | Drafts wait for the maintainer to review and file them. Leave them. |

## Limits

- The tool never closes or reopens an issue, never edits an issue's text, never deletes a board item, and never touches drafts. Do not do those here either.
- Labels are not used for status on this board. Do not add labels to stand in for a field.
- Board maintenance is bookkeeping. A driver at level 2 or above may apply it; at level 1, run the dry run and report it.
- Closing an issue is part of finishing work and follows the driver's level (`docs/delegation-levels.md`).
