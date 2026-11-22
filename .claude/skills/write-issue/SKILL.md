---
name: write-issue
description: Draft a GitHub issue for this repository in the maintainer's structure and voice. Use when asked to write, draft, rephrase or review an issue here, or to write up why a change is being made.
---

# Write an issue

Issues here explain why a change is being made, for a person who has not followed the work. They are read by the maintainers of ccc-server and the apps, not by agents.

## Structure

Always in this order:

1. **Opening paragraph, no heading.** State the problem or problems plainly, then what this issue tracks.
2. **`## Problems`** The facts behind the opening, each with a link to the code or issue that shows it. Leave this section out if the opening already covers it.
3. **`## Goals`** What should be true afterwards.
4. **`## Changes`** What will be done, at a high level. A numbered list when the order matters.
5. **`## Done when`** Acceptance criteria that someone can check.
6. **`## Open questions`** Only if there are real ones.

Titles are plain and name the area first where there is one: `Project Trifecta: Add caching`.

An issue that belongs to a larger one starts with `Part of #N.` on its own line, before the opening paragraph. Once it is filed, link it as a sub-issue of that issue (`gh api -X POST repos/frog-pond/ccc-server-next/issues/<parent>/sub_issues -F sub_issue_id=<the child's numeric id>`), and add its number where the parent mentions it. The rearchitecture's parent issue is #424.

## Short form for a piece of an approved issue

When an approved issue is split into pieces that add no new argument, such as one upstream's routes under #434, a piece may be filed directly without a draft. It has:

- `Part of #N.`
- one sentence saying what this piece is;
- `## Changes`, as a short list;
- `## Done when`.

Anything that makes a new argument, changes a goal or records a decision is a full issue and is drafted first.

## Voice

Write the way the maintainer does. This paragraph is his, and is the reference:

> The server currently has no caching nor singleflighting. This issue tracks adding both, through a shared pipeline framework which I'm calling "Trifecta". Notably, I want to preserve the ability to deploy on both Workers and a standalone docker image, so Trifecta is not just a caching and singleflighting framework but also a way of formalizing the pieces that we care about in application code and breaking things up. We also don't just want to have caching and singleflighting, we want to have some flexibility to proactively warm, backfill, and update the cache from dedicated worker jobs.

- First person singular for his views and intentions ("I want", "I'm calling").
- Flat, declarative statements of fact. Do not make it more exciting than it is: no scene-setting about phones or lunch queues, no slogans, no bold lead-in phrases, no rhetorical questions.
- Use the ordinary technical word: cache, singleflight, Workers, Durable Object, docker image, upstream. Do not paraphrase a standard term into everyday language, and do not invent a name for something. `Trifecta` is the one project name, and it means the caching and pipeline framework.
- Numbers and dates over adjectives: "19 routes", "open since October 2022", not "few" or "long-standing".
- Say what is not known as an open question. Do not hedge the rest.

## Links

- Link code with permalinks pinned to a commit, including the line or line range:
  `https://github.com/frog-pond/ccc-server/blob/<sha>/workers/src/source-do.ts#L108-L131`
- For ccc-server, use the commit in `docs/reference/REFERENCE_COMMIT`.
- For this repository, use the full commit of the branch the code is on (`git rev-parse origin/main` for code on `main`).
- Check each line number against that commit (`git show <sha>:<path>`) before using it.
- Refer to issues as `#13` here and `frog-pond/ccc-server#1119` across repositories.

## Process

1. Draft the issue and show it to the maintainer. Read it once for flow before showing it: each sentence should say one thing, and nothing should be there for effect.
2. When he approves the text, add it as a draft to the project "@rye's Rearchitecting Push" (`gh project item-create 4 --owner frog-pond --title "..." --body "..."`), where he reviews it in GitHub.
3. File it as a real issue only when he asks. The one exception is the short form above: once he has approved splitting an issue, its pieces are filed directly.
