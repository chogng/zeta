# Zeta

Zeta is a Rust-first agent workspace with three product lines sharing one App Server contract:

| Product | Description | Source | Start |
| --- | --- | --- | --- |
| `zeta code` | CLI and TUI | [`zeta-code`](zeta-code) | `just zeta` |
| `zeta` | Electron Desktop | [`zeta-ts`](zeta-ts) | `just zeta-desktop` |
| `app` | Native Rust Desktop terminal | [`app`](app) | `just app` |

`zeta-rs` contains the shared Rust backend. The product-neutral backend executable is
`zeta-server`, owned by [`zeta-server-host`](zeta-rs/server-host/README.md). Electron's `code` and
`academic` builds are internal variants, not additional product lines; see
[`docs/product-lines.md`](docs/product-lines.md) and [`docs/workbench-modes.md`](docs/workbench-modes.md).

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

Build definitions live in [`build/`](build), while reproducible local artifacts are collected under the ignored `.build/` root. See [`docs/build.md`](docs/build.md) for the command and output layout.

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

The Desktop command is shared by both Workbench build modes. The default mode is `code`; build
matrix checks can set `ZETA_WORKBENCH_MODE=academic` without changing the command name.

### Browser Workbench

```bash
corepack pnpm dev:web       # disconnected UI at http://127.0.0.1:5173/
corepack pnpm dev:web:full # Rust-backed UI at http://127.0.0.1:5174/
```

The full Web mode is a local development integration, not a deployable Web service.

### Stanza standalone editor

只调试 Stanza 编辑器本身时运行：

```bash
corepack pnpm dev:stanza
```

然后打开 `http://127.0.0.1:5199/build/vite/stanza/index.html`。在 VS Code 中也可以直接选择
`Stanza Editor - Standalone` 配置按 F5；它会自动启动同一个 Vite 任务。页面把完整 API 暴露为
`globalThis.stanza`，可在浏览器控制台检查 `stanza.editor.getEditors()` 和
`stanza.editor.getModels()`。

### `app`

```bash
just app
# or:
python3 -B build/cargo_with_v8.py run -p app
```

## Repository map

- [`zeta-rs`](zeta-rs): shared protocol, App Server, domain, storage, execution, and runtime crates.
- [`zeta-code`](zeta-code): CLI command host and TUI presentation.
- [`zeta-ts`](zeta-ts): Electron Main, Preload, Renderer, and Browser Workbench.
- [`build`](build): checked-in build orchestration; generated artifacts go to `.build/`.
- [`app`](app): native window, terminal, renderer, and product UI.
- [`docs`](docs): architecture and system documentation; start with [`docs/README.md`](docs/README.md).

## Where to read next

- [Product lines and host boundaries](docs/product-lines.md)
- [System architecture](docs/architecture.md)
- [CLI/TUI architecture](docs/zeta-cli-architecture.md)
- [Electron Desktop architecture](docs/zeta-desktop-architecture.md)
- [Shared Rust architecture](docs/zeta-rs-architecture.md)
- [Remote development](docs/remote-development.md)
- [Packaging](build/release/zeta_package/README.md)
- [`app` release graph](app/docs/app-release-graph.md)

Crate-level implementation details live in the `README.md` next to each crate.

## License

Zeta's original code and materials are proprietary and all rights reserved. See [`LICENSE`](LICENSE).
Third-party components remain governed by their own licenses and notices, including
[`zeta-ts/THIRD_PARTY_NOTICES.md`](zeta-ts/THIRD_PARTY_NOTICES.md).
