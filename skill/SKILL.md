---
name: dotagents
description: Universal compiler and reconciliation wizard for AI coding agent rules and MCP configs (Claude Code, Codex, Antigravity/Gemini, Cursor, Copilot). Trigger whenever a project has multiple unsynchronized agent files, after editing .dotagents/, or when setting up multi-agent rules in a repository.
---

# dotagents

Use `dotagents` to keep all AI coding agent configurations (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, `.github/copilot-instructions.md`, `.mcp.json`, `.codex/config.toml`) synchronized from a single `.dotagents/` source of truth.

## When to Run Automatically

1. **Entering a repository with multiple unsynchronized agent files** (e.g., separate `CLAUDE.md`, `AGENTS.md`, `.cursorrules`, `.mcp.json`).
2. **After modifying any file inside `.dotagents/`** (`rules/*.md` or `mcp.json`).
3. **Before committing** in a repository that uses `.dotagents/` to ensure zero configuration drift.

## Workflow

1. Check if `dotagents` is installed (`dotagents --version`). If missing, install via `cargo install --git https://github.com/alexisrazok/dotagents` or download from GitHub Releases.
2. **Existing project with unsynchronized agents**: run the interactive reconciliation wizard (creates `.dotagents/backup/` automatically, deduplicates rules, resolves conflicts, and merges MCPs):
   ```bash
   dotagents unify
   ```
   (Use `dotagents unify --auto` only when non-interactive defaults are explicitly desired.)
3. **New project**:
   ```bash
   dotagents init
   ```
4. **After editing `.dotagents/`**: regenerate all target agent files and verify no drift remains:
   ```bash
   dotagents generate
   dotagents generate --check
   ```
