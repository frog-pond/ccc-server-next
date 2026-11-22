# Running agents on different hosts

The skills in `.claude/skills/` (linked into `.agents/skills/` for Codex) describe what to do, not which program does it. They need five things from whatever runs them. This page says how to get each one in the hosts we use. T3 Code is the one the maintainer drives from; the others work too.

| What the skills need | T3 Code | Claude Code | Codex CLI | By hand |
|---|---|---|---|---|
| A **session** for the driver, where grants and questions live | A thread named `Driver: <the work>` | A named session (`claude -n "Driver: <the work>"`, resumed with `claude -r`) | A session, resumed with `codex resume` | Any conversation you keep |
| **Start a worker** on one issue, in its own checkout | Check which providers are available, then `delegate_task` with `isolation: "worktree"` | The Agent tool with `isolation: "worktree"`, or `claude -w <name> -p "<request>"` | `git worktree add` first, then `codex exec -C <worktree> -s workspace-write -c sandbox_workspace_write.network_access=true "<request>"` | `git worktree add ../ccc-server-next-<N> origin/<base>`, then start any agent there with the request |
| A **worker's report** | The child's final message, which wakes the driver | The subagent's result, or the output of `claude -p` | The output, or the file given to `codex exec -o <file>` | Whatever the agent printed last |
| A **reviewer from the other vendor** | `delegate_task` to the other provider | `codex review --base <branch>`, or `codex exec` with the review request | `claude -p` with the review request and the contents of `.claude/agents/arch-reviewer.md` | Either command, from the worktree being reviewed |
| The **previous driver's handoff** | Search threads for `Driver:` and read the last message | `claude -r` the previous driver session | `codex resume` the previous session | Ask the person who ran it for its last message |

## Rules that hold on every host

- The requests a driver sends are written out in full in `drive-rearchitecture`, so they can be pasted into any of the commands above.
- A worker works in its own git worktree and branches as `work-issue` says. Whatever created the worktree, the worker creates its branch from `origin/<base>`.
- A worker does not start other agents. If it needs a review or a second opinion, it says so in its report.
- Grants, handoffs and questions stay in the driver's session. If your host cannot read an earlier session, the next driver asks the maintainer for the last driver's final message. Nothing of this goes into issue or pull request comments.
- `tools/board`, and anything else that uses `gh`, needs network access and `gh` signed in with the `project` scope, on every host. A sandbox without network makes `gh` fail with `unknown owner type` or a connection error. Give the agent network access (for the Codex CLI, `-c sandbox_workspace_write.network_access=true`), or run the board yourself and pass on its output.
- Permissions differ between hosts. `.claude/settings.json` covers Claude Code only. On another host, follow the same limits by hand: never push to `main`, never force-push, and never edit the reference checkout.
