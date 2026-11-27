# Security Policy

## Reporting a Vulnerability

If you discover a security issue in `dotagents` (such as path traversal when generating target files or unsafe config serialization), please report it privately via GitHub Security Advisories.

## Security Design

- Generated target paths are strictly confined to known project/agent directories; rule names are sanitized to prevent path traversal (`..`, `/`, `\`).
- `dotagents` never transmits rules or MCP configs over the network.
- Portable `{{HOME}}` placeholders prevent leaking local usernames or absolute paths into shared repositories.
