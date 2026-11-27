# Changelog

## [0.1.0] - 2026-11-27

### Added
- Initial Rust release of `dotagents`.
- Interactive `dotagents unify` reconciliation wizard with automatic `.dotagents/backup/` before modifying existing agent configs, rule deduplication, conflict resolution, and MCP merging.
- Multi-agent compiler (`dotagents generate`) for Claude Code, Codex, Antigravity/Gemini, Cursor, and GitHub Copilot.
- Cross-OS `{{HOME}}` and Windows `cmd /c` MCP command wrapping, plus `--dry-run` and CI `--check` drift guard.
- `skill/SKILL.md` for AI agent integration.
