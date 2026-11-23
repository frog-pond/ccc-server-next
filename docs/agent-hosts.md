# Running agents on different hosts

The skills in `.claude/skills/` (linked into `.agents/skills/` for Codex) describe the work. This page says how to do it on each host. The maintainer drives from T3 Code.

The driver is Claude: coordinating is the work Claude does best here. Codex works issues and reviews. Any model can follow any skill, but this is the arrangement the procedures are tuned for.

| What the skills need | T3 Code | Claude app (Code tab) | Claude Code CLI | Codex CLI | By hand |
|---|---|---|---|---|---|
| A **session** for the driver | A thread named `Driver: <the work>` | A session named `Driver: <the work>` | A named session (`claude -n "Driver: <the work>"`, resumed with `claude --resume`) | A session, resumed with `codex resume` | Any conversation you keep |
| **Start a worker** on one issue, in its own checkout | Call `t3_thread_launch` with a title naming the issue, the request as `message`, and `workspaceStrategy: { type: "worktree", baseRef: "origin/<base>", startFromOrigin: true }` for the base the board prints. Pass a Codex `modelSelection` as shown below. Keep the returned thread ID and link. | Follow the Codex CLI recipe below. | Follow the Codex CLI recipe below. | Give the maintainer the launch command and request below to run in a terminal. | Use the Codex CLI recipe below. |
| A **worker's report** | Read the worker thread with `t3_thread_read`, even if it has no pull request. Use `t3_thread_wait` if you need to wait for it. | Ask the maintainer for the report from the worker session. | Ask the maintainer for the report from the worker session. | Read the report in the worker session if you can access it. Otherwise ask the maintainer for it. | Read the report in the worker session. |
| **Send a follow-up** to a worker | Use `t3_thread_send` with the worker's thread ID. | Give the maintainer the Codex CLI resume command and the follow-up request. | Give the maintainer the Codex CLI resume command and the follow-up request. | Give the maintainer the resume command and follow-up request below. | Return to the worker session and send the request. |
| A **reviewer from the other vendor** | `delegate_task` to the other provider | Run the review from a terminal using one of the CLI commands in this row. | `codex review --base <base>` with the pull request's base branch, or `codex exec` with the review request | `claude -p` with the review request and the contents of `.claude/agents/arch-reviewer.md` | Either command, from the worktree being reviewed |
| The **previous driver's handoff** | Search threads for `Driver:` and read the last message | Use [Work across sessions](https://code.claude.com/docs/en/desktop#work-across-sessions) to find the previous Code-tab driver session and read its last message. If you cannot read it, ask the maintainer to copy it. | Ask the maintainer to run `claude --resume "<driver session name or ID>"` and copy the last message. | Ask the maintainer to run `codex resume -C "<driver worktree>" "<driver session ID>"` and copy the last message. | Ask the person who ran it for its last message |

## Choose the Codex model in T3 Code

Use `orchestrator_capabilities` to find an available Codex provider instance and model. Pass its `providerInstanceId` as `instanceId`, its model ID as `model`, and any chosen options as `{ id, value }` entries. For example, when the catalog offers these values, pass:

```json
{
  "modelSelection": {
    "instanceId": "codex",
    "model": "gpt-6.2-sol",
    "options": [{ "id": "reasoningEffort", "value": "high" }]
  }
}
```

## Start or resume a Codex CLI worker

From the repository, run `git fetch origin`, then `git worktree add "../ccc-server-next-<N>" "origin/<base>"` for the base the board prints. If you cannot prepare the worktree yourself, give the maintainer both commands. Fill in the issue number, base and absolute worktree path in the commands you hand over. Give the maintainer the complete first request from `drive-rearchitecture` with the launch command:

```sh
codex -C "<worktree>" -s workspace-write
```

The maintainer pastes the request into that session. Once the session starts, ask the maintainer for its ID and record it with the worktree path. On exit, Codex prints a `codex resume` command with the ID. To read the report or give a follow-up, return to that session:

```sh
codex resume -C "<worktree>" "<session ID>"
```

## Rules that hold on every host

- Start each worker in a top-level session the maintainer can open and talk to directly. Keeping the conversation in that session makes it clear which pull request a finding or design question belongs to. Keep the link and thread ID in T3 Code, or the session name and ID where available, with the issue in the driver's handoff. Give the worker the request from `drive-rearchitecture`.
- A worker works in its own git worktree and branches as `work-issue` says.
- A worker does not start other agents. If it needs a review or a second opinion, it says so in its report.
- Grants, driver handoffs and the driver's own questions for the maintainer stay in the driver's session. Discuss an issue's design in the issue's session. If your host cannot read a session, ask the maintainer for its report or answer. Keep these conversations out of issue and pull request comments.
- `tools/board`, `git push` and `gh` need network access, and `gh` must be signed in with the `project` scope. A sandbox without network makes `gh` fail with `unknown owner type` or a connection error.
- When the sandbox blocks a command you need, ask for approval to run that one command outside it, using the approval mechanism your host gives you. Do not work around the sandbox, and do not skip the step without saying so. A non-interactive `codex exec` cannot ask, so give that run network access with `-c sandbox_workspace_write.network_access=true`. In a Codex thread started from T3 Code, this has been tested: Codex asked to run `tools/board` outside its sandbox, and Codex's automatic reviewer approved that one command. Who approves depends on the thread's runtime mode.
- `tools/board` calls `gh` itself. That is fine on a host that otherwise prefers its own tools for reading pull requests, such as T3 Code.
- Permissions differ between hosts. `.claude/settings.json` covers Claude Code only. On another host, follow the same limits by hand: never push to `main`, never force-push, and never edit the reference checkout.
