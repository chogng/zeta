# Zeta

Zeta is a Rust-first agent workspace with three product lines sharing one App Server contract:

| Product | Description | Source | Start |
| --- | --- | --- | --- |
| `zeta code` | CLI and TUI | [`zeta-code`](zeta-code) | `just zeta` |
| `zeta` | Electron Desktop | [`desktop`](desktop) | `just zeta-desktop` |
| `zeterm` | Native Rust Desktop terminal | [`zeterm`](zeterm) | `just zeterm` |

`zeta-rs` contains the shared Rust backend. The product-neutral backend executable is
`zeta-server`, owned by [`zeta-server-host`](zeta-rs/server-host/README.md). Electron's `code` and
`academic` builds are internal variants, not additional product lines; see
[`docs/product-lines.md`](docs/product-lines.md) and [`docs/product-editions.md`](docs/product-editions.md).

## Quick start

Install Rust and, for the Rust workspace's input classifier, Protocol Buffers:

```bash
# macOS
brew install protobuf

# Debian/Ubuntu
apt-get install protobuf-compiler
```

For Electron or Browser Workbench development, install the pinned pnpm workspace:

```bash
corepack pnpm install
```

### `zeta code`

```bash
just zeta
just zeta ask "explain this repository"
just zeta exec "summarize the current changes"
```

Without `just`:

```bash
cargo run -p zeta-cli --bin zeta
```

### `zeta` Electron Desktop

```bash
just zeta-desktop
# or:
corepack pnpm dev:desktop
```

Use `corepack pnpm dev:desktop:code` or `corepack pnpm dev:desktop:academic` to select a build
variant.

### Browser Workbench

```bash
corepack pnpm dev:web       # disconnected UI at http://127.0.0.1:5173/
corepack pnpm dev:web:full # Rust-backed UI at http://127.0.0.1:5174/
```

The full Web mode is a local development integration, not a deployable Web service.

### `zeterm`

```bash
just zeterm
# or:
cargo run -p zeterm
```

## Repository map

- [`zeta-rs`](zeta-rs): shared protocol, App Server, domain, storage, execution, and runtime crates.
- [`zeta-code`](zeta-code): CLI command host and TUI presentation.
- [`desktop`](desktop): Electron Main, Preload, Renderer, and Browser Workbench.
- [`zeterm`](zeterm): native window, terminal, renderer, and product UI.
- [`docs`](docs): architecture and system documentation; start with [`docs/README.md`](docs/README.md).
- [`scripts`](scripts): packaging and release tooling.

## Where to read next

- [Product lines and host boundaries](docs/product-lines.md)
- [System architecture](docs/architecture.md)
- [CLI/TUI architecture](docs/zeta-cli-architecture.md)
- [Electron Desktop architecture](docs/zeta-desktop-architecture.md)
- [Shared Rust architecture](docs/zeta-rs-architecture.md)
- [Remote development](docs/remote-development.md)
- [Packaging](scripts/zeta_package/README.md)
- [`zeterm` release graph](zeterm/docs/zeterm-release-graph.md)

Crate-level implementation details live in the `README.md` next to each crate.

## License

Zeta's original code and materials are proprietary and all rights reserved. See [`LICENSE`](LICENSE).
Third-party components remain governed by their own licenses and notices, including
[`desktop/THIRD_PARTY_NOTICES.md`](desktop/THIRD_PARTY_NOTICES.md).
