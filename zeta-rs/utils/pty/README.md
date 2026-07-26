# `zeta-utils-pty`

This is Zeta's local adaptation of Codex's PTY integration layer, pinned to the
revision recorded in `NOTICE`. It provides PTY and pipe process execution,
process-group cleanup, and Windows ConPTY support. It is source reuse only:
Zeta does not import a Codex protocol, RPC method, or runtime dependency.

The wrapper layer originates in Codex under Apache-2.0. The ConPTY files under
`src/win/` originate in WezTerm under MIT; both attributions are retained in
`NOTICE` and `third_party/wezterm/LICENSE`.

## API surface

- `spawn_pty_process()` creates an interactive terminal process.
- `spawn_pipe_process()` creates a non-interactive process with split pipes.
- `ProcessHandle` handles input, resize, interruption, and termination.
- `TerminalSize` defines PTY dimensions.
- `conpty_supported()` reports Windows ConPTY availability.
