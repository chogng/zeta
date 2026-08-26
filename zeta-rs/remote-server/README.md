# `zeta-remote-server`

`zeta-remote-server` is the headless runtime installed or selected on a target host. It owns a
per-user, per-Workspace broker that keeps one shared App Server alive across replacement SSH stdio
connections. It does not initiate SSH and has no desktop or TUI presentation responsibilities.

## Invocation

The normal connection command is:

```bash
ZETA_WORKSPACE_ROOT=/absolute/remote/workspace \
zeta-remote-server remote-server connect
```

An installed full `zeta code` runtime exposes the same boundary as `zeta remote-server connect`.
The direct `app-server --listen stdio://` command remains available for diagnostics and
compatibility, but it is process-scoped and cannot preserve a PTY after its stdio process exits.

`ZETA_WORKSPACE_ROOT` must be absolute. `ZETA_PROFILE_ROOT` optionally selects durable remote
state; otherwise the runtime uses a per-user `remote-server` state directory under the host's
normal platform state location.

This binary is optional. A Remote host may instead use the `zeta` executable from an installed
`zeta code` CLI. The host-side connection layer selects the executable and does not require both
binaries to be installed.

## Execution path

```text
Remote connection host
  -> ssh … zeta-remote-server remote-server connect
  -> private Unix socket lookup / guarded daemon start
  -> stdio proxy
  -> shared RemoteServerOptions + AppServer daemon
  -> zeta_app_server::open_local_app_server
  -> AppServer::serve_jsonl for each connection
```

The endpoint identity hashes the canonical profile root, canonical Workspace root, canonical
runtime executable generation (path plus Unix file identity and timestamps), selected
product-services manifest identity, and App Server schema. Rebuilding a development executable at
the same path therefore starts a new broker generation instead of reconnecting to an older daemon;
runtime or product configuration generations cannot accidentally reuse an older daemon merely
because their schemas match. Its directory is real, owned by the
effective user, and mode `0700`; the socket and log are private. A guarded start lock prevents
duplicate daemon creation. The daemon remains alive while clients or PTYs exist and otherwise
exits after a bounded idle period. A reconnectable terminal is detached for 30 seconds when its
connection closes, accepts only its 256-bit bearer token, and rotates that token after a successful
attach.

`RemoteServerOptions` owns the remote profile and Workspace roots plus an optional manifest path
selected by the executable host. The full `zeta` host supplies its packaged product-services
manifest; the standalone binary supplies none by default. Manifest discovery, runtime download,
activation, rollback, SSH retry, and tunnel policy stay outside this crate.

## Failure semantics

Unsupported command arguments, a missing or relative `ZETA_WORKSPACE_ROOT`, an unsafe runtime
directory, a conflicting endpoint, or daemon startup timeout return `RemoteServerError` before a
connection is exposed. Per-connection protocol failures are written only to the private daemon
log. SSH credentials cannot be exposed because SSH is not present in this process.

## Extension direction

Add persistent session identity, host-restart restoration, capability reporting, and logical
tunnel endpoints for non-SSH transports or Remote service discovery as explicit protocol work.
Basic local `ssh -L` forwarding does not traverse this daemon: keep its listener, credentials, and
SSH spawning in `zeta-remote-connections`, its shared lifecycle supervision in `zeta-remote-host`,
and UI/product composition in the product host.

## Verification

```bash
cargo test -p zeta-remote-server
```

The integration tests start the binary and connect through `AppServerSession::start_stdio` to
cover direct stdio plus the real broker process boundary. They prove that a reconnectable PTY
survives the first connector process and can be closed by the replacement connection. App Server
integration tests separately prove wrong-token rejection and old-token replay rejection after
rotation.
