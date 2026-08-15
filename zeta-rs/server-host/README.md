# `zeta-server-host`

`zeta-server-host` owns the product-neutral executable boundary used by Desktop and other product
clients to start shared Rust backend capabilities. Its binary is `zeta-server`; it does not contain
Zeta Code CLI/TUI composition. The canonical cross-product consumer boundary is documented in
[`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md).

## Ownership and contract

`src/lib.rs` is the public composition boundary. `run` dispatches the stable process namespaces,
while `run_app_server` and `run_remote` preserve compatibility for product hosts that already parse
their own top-level arguments. `src/main.rs` only maps process success and failure to an exit code.

`src/app_server.rs::run` validates the stdio listener arguments, binds `ZETA_PROFILE_ROOT`, binds a
Workspace only when `ZETA_WORKSPACE_ROOT` is explicitly present, resolves the optional product
services manifest through `InstallContext`, and delegates all server behavior to
`zeta-app-server`. It must never infer a Workspace from the host process current directory.

`src/remote.rs::run` owns the non-interactive Remote command adapter used by Desktop: runtime probe,
download and installation, connection catalog operations, and runtime profile operations. Domain
validation and SSH behavior remain in `zeta-remote` and `zeta-remote-connections`. Interactive
`remote connect` stays in `zeta-cli` because it composes the Zeta Code TUI.

## Failure semantics

Unknown or malformed commands fail before opening an App Server or starting SSH. Domain failures
are returned as text to the executable boundary, printed to stderr, and produce a failing exit code.
The App Server inherits no implicit Workspace authority; without `ZETA_WORKSPACE_ROOT`, Workspace
operations report their canonical unavailable errors.

## Integration obligations

Desktop packages `bin/zeta-server[.exe]` and invokes `app-server --listen stdio://` or the supported
`remote` commands. Product compatibility commands may delegate to this crate, but product clients
must not package `zeta-cli` as their backend. Adding TUI state, interactive prompts, renderer policy,
or product-specific command discovery here is architectural drift.

## Tests and extension points

Run `cargo test -p zeta-server-host`. Unit tests live beside the Remote command modules; stdio
integration tests exercise the real binary and verify both explicit and empty Workspace behavior.
Add a new command only when it is backend-neutral and has a canonical shared domain owner. Product
commands remain in their product host.
