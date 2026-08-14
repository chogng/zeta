# `zeta-code`

`zeta-code` is the product-owned source boundary for the `zeta code` terminal product.
It contains the `zeta-cli` command host and the `zeta-tui` presentation shell.

The product depends on shared contracts and runtime services from `zeta-rs`, but the terminal
experience itself does not belong to the shared backend. Raw mode, alternate-screen lifecycle,
keyboard input, Ratatui layout, composer state, and TUI presentation state stay here.

```text
zeta-code/cli  → zeta-code/tui → zeta-app-server-client → shared App Server crates
```

The native `zeterm` product has a separate ownership boundary under `zeterm/`; its reusable GPU UI
crates, including `zeta-ui` and `zui`, must not be copied into this product.

The CLI also exposes the native-host Remote management entrypoints used by Desktop and operators:

```text
zeta remote connect -> resolve a saved/direct target and open the TUI over host-owned OpenSSH
zeta remote probe   -> detect the exact POSIX package target through local OpenSSH
zeta remote install -> validate a trusted local packaged-node artifact and install it immutably
zeta remote profile -> read, activate, or compatibility-check and roll back shared runtime history
```

These commands delegate SSH/package/profile semantics to `zeta-remote-connections`. For
`zeta remote connect`, the CLI product host owns OpenSSH and gives the transport-neutral TUI an
already initialized `AppServerSession`; the TUI never owns credentials or process launch.
`--name` resolves the shared credential-free target catalog, while direct `--host`/`--workspace`
uses the same path. A managed connection first tries the stored exact runtime or Remote `zeta`.
For runtime-unavailable and protocol-incompatible failures only, it can load the authenticated
catalog bound by a packaged `zeta code` installation, or an explicit local/HTTPS catalog plus
SHA-256, install the matching immutable package, and retry once. An explicit `--runtime` is never
replaced automatically. The selected runtime is activated only after executable resolution and
the protocol/schema handshake succeed. `--check` performs the same chain without requiring a TTY
and exits after a clean shutdown. Downloads remain local and the installer uploads an already
validated package; no artifact URL or credential is sent to the Remote host.
`zeta remote install --progress json-lines` writes typed installation phases to stderr
while keeping stdout reserved for the final immutable executable path. Native product hosts may
terminate this local command to cancel bootstrap; Desktop does so from its Main-owned pre-Workbench
progress window without exposing artifact paths, SSH options, or credentials to Renderer.

After an interactive SSH TUI has started, a connection loss returns only the durable Session and
Thread identity to the CLI host. The host retries the same verified runtime for 30 seconds with
250ms-to-2s backoff, then starts a fresh TUI connection and reloads the canonical Thread snapshot.
Requests that were in flight and actions queued behind them are discarded rather than replayed.
Runtime disappearance, schema changes, protocol stream failures, and server rejection stop
recovery immediately. The Remote workspace is displayed in the TUI, while local `@file` scanning
is disabled so a local checkout cannot be projected into the Remote conversation; Remote path
completion awaits an App Server-owned contract.

An unpackaged development build has no implicit release trust binding. Use the existing local
bundle explicitly when exercising automatic preparation:

```bash
zeta remote connect --name work \
  --runtime-catalog /absolute/path/to/catalog.json \
  --runtime-catalog-sha256 <catalog-sha256> \
  --check
```

Run the product from the repository root:

```bash
cargo run --manifest-path Cargo.toml -p zeta-cli
cargo test --manifest-path Cargo.toml -p zeta-cli
cargo test --manifest-path Cargo.toml -p zeta-tui
bazel test //zeta-code/tui:tui-unit-tests
```

`zeta-code/cli/tests/remote_connect.rs` exercises target resolution, the real local Remote Server
broker, trusted runtime preparation, and `--check` through a fake OpenSSH executable.
`zeta-code/cli/tests/remote_connect_interactive.rs` runs the real CLI/TUI in a PTY, cuts the first
SSH proxy after the TUI is ready, proves the replacement generation reads the durable Session and
Thread, and exits through the terminal input path.
