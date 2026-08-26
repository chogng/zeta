# `zeta-remote`

`zeta-remote` owns the product-neutral identity of an SSH Remote target. It does not start SSH,
store credentials, decide how a runtime is installed, or render a Remote UI.

## Contract

`RemoteProfile` combines exactly one `SshTarget` with one `RemoteRuntime`.

- `SshHost::parse` accepts only an OpenSSH host alias. User names, passwords, shell syntax, and
  control characters never enter the profile.
- `RemoteWorkspacePath::parse` accepts one canonical absolute POSIX Workspace root.
- `RemoteRuntime::new` identifies a runtime selected by an installer or an already-compatible
  `zeta code` CLI. It is an executable reference, not a product identity.
- `RemotePlatform` represents only the currently supported POSIX package targets: Linux GNU/musl
  and macOS on `aarch64`/`x86_64`. It cannot represent a Linux target without a libc ABI.

The profile is deliberately credential-free. A product host passes its SSH agent and OpenSSH
configuration only when it asks `zeta-remote-connections` to connect.

## Execution path

```text
product profile store
  -> RemoteProfile
  -> zeta-remote-connections::SshAppServerConnectionOptions
  -> local OpenSSH child
```

`zeta-remote-server` consumes the Workspace root after SSH reaches the target; this crate has no
dependency on either implementation.

## Failure semantics

Invalid hosts, Workspace paths, and executable references fail before any process is launched as
`RemoteAddressError`. A valid profile does not prove that the host is reachable or that the
runtime exists; those are connection and installation concerns.

## Extension direction

Add another target transport only when it has a distinct authority contract. Keep product labels,
credentials, SSH process options, installer state, and tunnel lifecycle outside this crate; shared
local Tunnel lifecycle coordination belongs to `zeta-remote-host`.

## Verification

```bash
cargo test -p zeta-remote
```
