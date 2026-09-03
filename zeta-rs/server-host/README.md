# `zeta-server-host`

`zeta-server-host` owns the product-neutral executable boundary used by Desktop and other product
clients to start shared Rust backend capabilities. Its binary is `zeta-server`; it does not contain
Zeta Code CLI/TUI composition. The canonical cross-product consumer boundary is documented in
[`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md).

## Ownership and contract

`src/lib.rs` is the public composition boundary. `run` dispatches the stable process namespaces,
while `run_app_server` and `run_remote` preserve compatibility for product hosts that already parse
their own top-level arguments. `src/main.rs` only maps process success and failure to an exit code.

`src/app_server.rs::run` validates stdio, loopback WebSocket, broker-connect, and daemon lifecycle commands; binds `ZETA_PROFILE_ROOT`; binds a directory only when `ZETA_WORKSPACE_ROOT` is explicitly present; resolves the optional product services manifest through `InstallContext`; and delegates server behavior to `zeta-app-server`. It must never infer a directory from the host process current directory. The WebSocket command passes a capability-token digest to `zeta-app-server-transport`, then emits exactly one generated `AppServerListenInfo` JSON line after the listener is bound.

`zeta-app-server-daemon` owns the local profile authority process, serialized lifecycle operations,
private socket paths, bounded data/control preludes, process-generation records, initialize/schema
readiness probes, stdio proxy, bounded logs, cooperative stop, idle shutdown, and stale-state recovery.
`app-server connect` resolves its packaged executable from `ZETA_APP_SERVER_DAEMON_PATH`, or from
the `zeta-server` sibling path for standalone and Remote runtime packages. The daemon crate's
[`README`](../app-server-daemon/README.md) defines its profile-wide and directory-scoped ownership.

`src/remote.rs::run` owns the non-interactive Remote command adapter used by Desktop: runtime probe,
download and installation, connection catalog operations, and runtime profile operations. Domain
validation and SSH behavior remain in `zeta-remote` and `zeta-remote-connections`. Interactive
`remote connect` stays in `zeta-cli` because it composes the Zeta Code TUI.

## Failure semantics

Unknown or malformed commands fail before opening an App Server or starting SSH. Domain failures
are returned as text to the executable boundary, printed to stderr, and produce a failing exit code.
The App Server inherits no implicit directory authority; without `ZETA_WORKSPACE_ROOT`, directory
operations report their canonical unavailable errors. A Session mutation whose durable directory
binding differs from the connection runtime fails with `WorkspaceAuthorityMismatch`; legacy
unbound Sessions remain readable but cannot be mutated implicitly.

## Integration obligations

The supported shared-process entrypoint is `app-server --listen ws://127.0.0.1:0 --ws-auth capability-token --ws-token-sha256 HEX --emit-listen-info stdout-json`. Each client connection uses the emitted loopback endpoint and the original token; the digest, not the token, crosses the process argument boundary. Desktop still invokes `app-server connect` until its connection relay migrates to this entrypoint. `app-server --listen stdio://` remains the direct compatibility/test mode, while `app-server connect` and the sibling daemon remain available to clients that need profile process reuse. Product clients must not package `zeta-cli` as their backend. Adding TUI state, interactive prompts, renderer policy, or product-specific command discovery here is architectural drift.

## Tests and extension points

Run `just test zeta-server-host` and `just test zeta-app-server-daemon`. Server-host tests exercise stdio compatibility, WebSocket argument validation, two independently initialized connections, and connection-local close behavior. Daemon tests exercise the profile endpoint, concurrent startup election, idempotent lifecycle commands, generation replacement, and initialize gate. Add a new command only when it is backend-neutral and has a canonical shared domain owner. Product commands remain in their product host.
