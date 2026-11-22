@AGENTS.md

## Claude Code specifics

- Skills live in `.claude/skills/` and are linked into `.agents/skills/` for Codex. Edit the files under `.claude/skills/`.
- The `arch-reviewer` subagent (`.claude/agents/arch-reviewer.md`) is the fresh-context Claude reviewer the `arch-review` skill calls for.
- Project permissions are in `.claude/settings.json`. Rules that name the reference checkout by absolute path are in `.claude/settings.local.json`, which is not committed and is absent from worktrees.
