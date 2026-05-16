# Research: Codex / Claude Code 初始化入口

## Codex

- Codex CLI `/init` creates `AGENTS.md`.
- Codex reads `AGENTS.md` for project-specific instructions.
- Codex project assets can use `.codex/agents`, `.agents/skills`, and MCP config.

## Claude Code

- Claude Code `/init` creates `CLAUDE.md`.
- Claude Code reads `CLAUDE.md` as project memory.
- Claude Code project assets commonly use `CLAUDE.md`, `.claude/agents`, `.claude/settings.json`, and MCP config.

## Decision

Generate one native primary entry per target client in v1:

- `codex` -> `AGENTS.md`
- `claude-code` -> `CLAUDE.md`
