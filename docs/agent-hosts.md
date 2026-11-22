# Running agents on different hosts

The skills in `.claude/skills/` (linked into `.agents/skills/` for Codex) describe the work. This page says how to do it on each host. The maintainer drives from T3 Code.

The driver is Claude: coordinating is the work Claude does best here. Codex works issues and reviews. Any model can follow any skill, but this is the arrangement the procedures are tuned for.

| What the skills need | T3 Code | Claude app (Code tab) | Claude Code CLI | Codex CLI | By hand |
|---|---|---|---|---|---|
| A **session** for the driver | A thread named `Driver: <the work>` | A session named `Driver: <the work>` | A named session (`claude -n "Driver: <the work>"`, resumed with `claude -r`) | A session, resumed with `codex resume` | Any conversation you keep |
| **Start a worker** on one issue, in its own checkout | Call `t3_thread_launch` with a title naming the issue, the request as `message`, and `workspaceStrategy: { type: "worktree", baseRef: "origin/<base>", startFromOrigin: true }` for the base the board prints. Keep the returned thread link. | Start a new session with Worktree enabled, give it the request, and name it for the issue. | In the issue's worktree, run `claude -n "Issue #<N>: <title>" "<request>"`. | Start an interactive session with `codex -C <worktree> -s workspace-write "<request>"`. | `git worktree add ../ccc-server-next-<N> origin/<base>`, then open a named session there with the request. |
| A **worker's report** | Read the worker thread's final message with `t3_thread_read`. Use `t3_thread_wait` if you need to wait for it. | Open the issue's session in the sidebar and read its final message. | Read the final message in the issue's session. | Read the final message in the issue's session. | Read the final message in the issue's session. |
| **Send a follow-up** to a worker | Use `t3_thread_send` with the worker's thread ID. | Open the issue's session in the sidebar and send the request there. | Use the open session or resume it with `claude -r` and send the request. | Use the open session or pick it with `codex resume` from its worktree and send the request. | Return to the issue's session and send the request. |
| A **reviewer from the other vendor** | `delegate_task` to the other provider | Run the review from a terminal using one of the CLI commands in this row. | `codex review --base <branch>`, or `codex exec` with the review request | `claude -p` with the review request and the contents of `.claude/agents/arch-reviewer.md` | Either command, from the worktree being reviewed |
| The **previous driver's handoff** | Search threads for `Driver:` and read the last message | Open the previous driver session in the sidebar and read its last message. | `claude -r` the previous driver session | `codex resume` the previous session | Ask the person who ran it for its last message |

## Rules that hold on every host

- Start each worker in a top-level session the maintainer can open and talk to directly. Keeping the conversation in that session makes it clear which pull request a finding or design question belongs to. Keep the link or session name with the issue in the driver's handoff. Paste the request from `drive-rearchitecture` into that session.
- A worker works in its own git worktree and branches as `work-issue` says. For the CLI hosts, prepare the worktree as in the "By hand" column before starting the session.
- A worker does not start other agents. If it needs a review or a second opinion, it says so in its report.
- Grants and driver handoffs stay in the driver's session. Discuss an issue's design in the issue's session. If your host cannot read an earlier driver session, ask the maintainer for its final message. Keep these conversations out of issue and pull request comments.
- `tools/board`, `git push` and `gh` need network access, and `gh` must be signed in with the `project` scope. A sandbox without network makes `gh` fail with `unknown owner type` or a connection error.
- When the sandbox blocks a command you need, ask for approval to run that one command outside it, using the approval mechanism your host gives you. Do not work around the sandbox, and do not skip the step without saying so. A non-interactive `codex exec` cannot ask, so give that run network access with `-c sandbox_workspace_write.network_access=true`. In a Codex thread started from T3 Code, this has been tested: Codex asked to run `tools/board` outside its sandbox, and Codex's automatic reviewer approved that one command. Who approves depends on the thread's runtime mode.
- `tools/board` calls `gh` itself. That is fine on a host that otherwise prefers its own tools for reading pull requests, such as T3 Code.
- Permissions differ between hosts. `.claude/settings.json` covers Claude Code only. On another host, follow the same limits by hand: never push to `main`, never force-push, and never edit the reference checkout.
