# dotagents

Compilador universal en Rust para mantener una única fuente de verdad (`.dotagents/`) de reglas, instrucciones y servidores MCP, generando los archivos nativos de Claude Code, Codex, Antigravity/Gemini, Cursor y GitHub Copilot.

## Características

- **Fuente única (`.dotagents/rules/*.md` y `.dotagents/mcp.json`)**: escribe tus reglas una sola vez con frontmatter (`targets`, `globs`, `description`).
- **Compilación multi-agente (`dotagents generate`)**: genera `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, `.github/copilot-instructions.md`, `.mcp.json`, `.cursor/mcp.json` y `.codex/config.toml`.
- **Portabilidad entre sistemas**: traduce `{{HOME}}` y envuelve comandos como `npx` con `cmd /c` en Windows automáticamente.
- **Guardia de drift en CI (`--check`)**: falla con código 1 si alguien editó un archivo generado sin actualizar `.dotagents/`.

## Instalación

```bash
cargo install --git https://github.com/alexisrazok/dotagents
```

## Uso

```bash
# En un proyecto existente con varios agentes desincronizados (asistente interactivo):
dotagents unify
# O en modo automático sin preguntas:
dotagents unify --auto

# Inicializar .dotagents/ desde cero o importar reglas existentes
dotagents init
dotagents import

# Generar archivos para todos los agentes
dotagents generate

# Previsualizar o verificar en CI/CD
dotagents generate --dry-run
dotagents generate --check
dotagents status
```

## Tests

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```
