# Zeta

Zeta is a Rust-first agent system with CLI, TUI, and app-server product entries.

The Rust workspace lives in [`zeta-rs`](zeta-rs); Electron is a separate client under
[`desktop`](desktop).

Team responsibilities and integration contracts are documented separately:

- [Desktop architecture](docs/zeta-desktop-architecture.md)
- [CLI architecture](docs/zeta-cli-architecture.md)
- [zeta-rs architecture and public surfaces](docs/zeta-rs-architecture.md)
- [Accepted App Server API v1](docs/zeta-app-server-api-v1.md)
- [API contract requirements](docs/zeta-api-interface-requirements.md)
- [API contract template](docs/zeta-api-interface-template.md)

The original
[`zeta-code-architecture-codex-style-v2.md`](docs/zeta-code-architecture-codex-style-v2.md)
is retained as the historical unified design.

## Run

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- ask "explain this repository"
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- exec "summarize the current changes"
```
