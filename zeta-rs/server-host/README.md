# `zeta-server-host`

`zeta-server-host` owns the product-neutral executable boundary used by Desktop and other product
clients to start shared Rust backend capabilities. Its binary is `zeta-server`; it does not contain
Zeta Code CLI/TUI composition. The canonical cross-product consumer boundary is documented in
[`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md).

## Ownership and contract

`src/lib.rs` is the public composition boundary. `run` dispatches the stable process namespaces,
while `run_app_server` and `run_remote` preserve compatibility for product hosts that already parse
their own top-level arguments. `src/main.rs` only maps process success and failure to an exit code.

`src/app_server.rs::run` validates direct, broker-connect, and daemon commands; binds
`ZETA_PROFILE_ROOT`; binds a Workspace only when `ZETA_WORKSPACE_ROOT` is explicitly present;
resolves the optional product services manifest through `InstallContext`; and delegates server
behavior to `zeta-app-server`. It must never infer a Workspace from the host process current
directory.

`src/app_server_broker.rs` owns the local profile authority process. Its endpoint identity includes
only the canonical profile root, host version, and protocol schema; Workspace and product-services
identities are carried in a bounded connection prelude instead of partitioning the daemon. One
`ProfileAppServerRegistry` reuses a single `LocalProfileRuntime` and lazily composes an isolated
`AppServer` runtime per canonical Workspace/trust-source/product-adapter key. Session/Thread
projections, Config, Marketplace instances for equivalent authorities, and Session notifications
are profile-wide; filesystem, Git, language, document, terminal, and execution runtime remain
Workspace-scoped. The broker also owns daemon election, private
Unix-domain socket paths, stdio proxying, bounded logs, idle shutdown, and stale-start recovery.

`src/remote.rs::run` owns the non-interactive Remote command adapter used by Desktop: runtime probe,
download and installation, connection catalog operations, and runtime profile operations. Domain
validation and SSH behavior remain in `zeta-remote` and `zeta-remote-connections`. Interactive
`remote connect` stays in `zeta-cli` because it composes the Zeta Code TUI.

## Failure semantics

Unknown or malformed commands fail before opening an App Server or starting SSH. Domain failures
are returned as text to the executable boundary, printed to stderr, and produce a failing exit code.
The App Server inherits no implicit Workspace authority; without `ZETA_WORKSPACE_ROOT`, Workspace
operations report their canonical unavailable errors. A Session mutation whose durable Workspace
binding differs from the connection runtime fails with `WorkspaceAuthorityMismatch`; legacy
unbound Sessions remain readable but cannot be mutated implicitly.

## Integration obligations

Desktop packages `bin/zeta-server[.exe]` and invokes `app-server connect`; TUI and zeterm expose the
same hidden command by delegating it to this crate. `app-server --listen stdio://` remains the direct,
single-process compatibility/test mode. Product compatibility commands may delegate here, but
product clients must not package `zeta-cli` as their backend. Adding TUI state, interactive prompts,
renderer policy, or product-specific command discovery here is architectural drift.

## Tests and extension points

Run `cargo test -p zeta-server-host`. Unit tests live beside the broker and Remote command modules;
stdio integration tests exercise the real binary, explicit and empty Workspace behavior, and live
Session visibility across two product connections.
Add a new command only when it is backend-neutral and has a canonical shared domain owner. Product
commands remain in their product host.
