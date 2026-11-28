---
name: dotagents
description: Universal compiler and reconciliation wizard for AI coding agent rules and MCP configs (Claude Code, Codex, Antigravity/Gemini, Cursor, Copilot). Trigger whenever a project has multiple unsynchronized agent files, after editing .dotagents/, when switching agents mid-project, or when setting up multi-agent rules in a repository.
---

# dotagents

Use `dotagents` to keep all AI coding agent configurations (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, `.github/copilot-instructions.md`, `.mcp.json`, `.codex/config.toml`) synchronized from a single `.dotagents/` source of truth.

## Strict Rule for Agents

- **Never edit `CLAUDE.md`, `GEMINI.md`, `AGENTS.md`, or `.cursor/rules/*.mdc` directly** when `.dotagents/` exists in a repository.
- Always add or update rules inside `.dotagents/rules/*.md` (and MCP servers inside `.dotagents/mcp.json`), then run `dotagents generate` immediately so every other agent stays in sync.

## When to Run Automatically

1. **When switching agents mid-project** (e.g., quota exhausted on one agent): run `dotagents sync` first to absorb any direct edits made by the previous agent and propagate them everywhere without losing changes.
2. **Entering a repository with multiple unsynchronized agent files**: run `dotagents unify`.
3. **After modifying any file inside `.dotagents/`**: run `dotagents generate` and `dotagents generate --check`.

## Workflow

1. Check if `dotagents` is installed (`dotagents --version`). If missing, install via `cargo install dotagents-cli` or download from GitHub Releases.
2. **Switching agents / quick two-way sync**:
   ```bash
   dotagents sync
   ```
3. **Existing project with unsynchronized agents**:
   ```bash
   dotagents unify
   ```
4. **New project**:
   ```bash
   dotagents init
   ```
5. **After editing `.dotagents/`**:
   ```bash
   dotagents generate
   dotagents generate --check
   ```
