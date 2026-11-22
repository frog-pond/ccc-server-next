# Delegation levels

The maintainer grants the driver one level for each push of work: a set of issues, up to an end point. The driver recommends a level, says why, and asks. The maintainer may grant that level or another, and may raise or lower it at any time. Work with no grant runs at level 1.

Reading needs no grant at any level: the board, issues, pull requests, the reference and earlier sessions. It is how the driver chooses what to recommend.

The scale follows the common five levels of delegation, from "do exactly what I asked" to "act and report".

| Level | Name | The driver may | It stops before | The maintainer hears |
|---|---|---|---|---|
| 1 | Do as told | Take only the step the maintainer named, such as starting a worker on one issue. Every question goes to the maintainer. | Anything not named | After the step |
| 2 | Look and report | Run reviews, keep the board in order (`board-maintenance`), and draft the requests it would send to workers. | Starting a worker | What it found, and the drafts |
| 3 | Recommend | Start workers and arrange reviews. Bring each worker question, each set of review findings and each pull request that is ready to merge to the maintainer as options with a recommendation, then carry out the maintainer's choice. | Every ruling, and every merge | At each decision |
| 4 | Act, stop before merge | Start workers, answer their questions from `AGENTS.md`, the review rules and the issues, act on review findings, and split an approved issue into pieces. | Every merge | A summary at the end point, with "Decisions I made" |
| 5 | Act and report | Everything in level 4. Also merge pull requests into `rearchitecture`, bottom of the stack first, and close the issues whose "Done when" is met. | The end point | A summary at the end point, with "Decisions I made" |

At level 5 a pull request merges only when its checks pass and its review covers its head. A pull request that got the thorough review also needs the maintainer's approval on GitHub, so at level 5 the driver merges it once the maintainer has approved it.

## Escalated at every level

No level covers these. They go to the maintainer even at level 5:

- anything that pushes to `main`, opens a pull request against it or merges into it, and rewriting pushed history;
- a difference from the reference, deliberate or found necessary (rule P0);
- the types for the first route that reads from an upstream;
- a new workspace crate, or an outside dependency the issue does not name;
- the two reviewers disagreeing about structure, or a finding the driver disagrees with;
- a change to "How the server is built" or the rules in `AGENTS.md`, to `docs/review/rubric.md`, to `docs/delegation-levels.md`, or to permissions under `.claude/`;
- filing a full issue, or changing an issue's goals or "Done when";
- the reference having moved (`tools/reference-drift`);
- capturing a new fixture from a live upstream;
- deploying, setting secrets, publishing a crate, or anything in the Cloudflare account.

## Choosing the recommendation

The driver looks at how much the work fixes in place that later work will build on, and recommends:

- **Level 3 or lower** for Trifecta's public traits (#425); the Workers experiment and what it decides (#429); the type conventions and versioning (#430); the first route for an upstream; and any issue with open questions.
- **Level 4** for work whose design is settled: CI checks (#428), fixtures and the comparison tests (#431), events once their names are agreed (#432), and further routes for an upstream whose types are approved.
- **Level 5** for documentation and tooling fixes, redirects and fixed notices, and fixes to a pull request that has already been reviewed.

The recommendation names the signals behind it in one line.

## Asking

The driver asks "What level can I run at?" with two to four levels that make sense for the work, the recommended one first. Each option is written for the work at hand, saying what it would do and where it would stop, for example: "Level 4: start workers on #428 and #431, act on review findings, stop when both pull requests are ready to merge."

## Recording

A grant is given in the driver's session (in T3 Code, its thread) and stays there. The driver states it back in one line: the level, the issues it covers, the end point, and any escalations beyond the fixed list.

A grant covers the session it was given in. A new driver session asks again, unless the maintainer points it to the session that holds a grant still in force. The driver's last message in each session lists the level of each piece of work in flight. When the maintainer changes a level, the driver states the new grant the same way.
