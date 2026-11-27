# dotagents

[![CI](https://github.com/alexisrazok/dotagents/actions/workflows/ci.yml/badge.svg)](https://github.com/alexisrazok/dotagents/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/alexisrazok/dotagents)](https://github.com/alexisrazok/dotagents/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

Compilador universal en Rust para mantener una única fuente de verdad (`.dotagents/`) de reglas, instrucciones y servidores MCP, generando los archivos nativos de Claude Code, Codex, Antigravity/Gemini, Cursor y GitHub Copilot.

## Características

- **Asistente para proyectos existentes (`dotagents unify`)**: analiza proyectos ya configurados con múltiples agentes desincronizados, hace backup en `.dotagents/backup/`, elimina reglas duplicadas, resuelve conflictos de forma interactiva y unifica servidores MCP.
- **Fuente única (`.dotagents/rules/*.md` y `.dotagents/mcp.json`)**: escribe tus reglas una sola vez con frontmatter (`targets`, `globs`, `description`).
- **Compilación multi-agente (`dotagents generate`)**: genera `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.cursor/rules/*.mdc`, `.github/copilot-instructions.md`, `.mcp.json`, `.cursor/mcp.json` y `.codex/config.toml`.
- **Portabilidad entre sistemas**: traduce `{{HOME}}` y envuelve comandos como `npx` con `cmd /c` en Windows automáticamente.
- **Guardia de drift en CI (`--check`)**: falla con código 1 si alguien editó un archivo generado sin actualizar `.dotagents/`.

## Instalación

### Binarios precompilados (Linux, macOS, Windows)
Descarga el binario correspondiente a tu sistema operativo desde la página de [Releases](https://github.com/alexisrazok/dotagents/releases/latest).

### Desde crates.io con Cargo
```bash
cargo install dotagents-cli
```

## Uso

```bash
# En un proyecto existente con varios agentes desincronizados:
dotagents unify
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

## Desarrollo, Seguridad y Licencia

- Guía de contribución: [CONTRIBUTING.md](CONTRIBUTING.md)
- Política de seguridad: [SECURITY.md](SECURITY.md)
- Historial de cambios: [CHANGELOG.md](CHANGELOG.md)
- Licencia: [Apache-2.0](LICENSE) y [NOTICE](NOTICE)
