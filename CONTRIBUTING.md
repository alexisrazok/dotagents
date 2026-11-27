# Contributing

Thank you for contributing!

## Development Setup

1. Install Rust via [rustup](https://rustup.rs/).
2. Run formatting, linting, and tests before opening a PR:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Guidelines

- Keep changes focused and add integration tests for new features or bug fixes.
- Ensure cross-platform compatibility across Linux, macOS, and Windows.
- Follow the [Apache-2.0](LICENSE) license terms.
