# `zeta-terminal-workspace`

`zeta-terminal-workspace` owns the process-local mapping between Workbench sessions, terminal keys, pending startup, and active or inactive terminal runtimes. It also owns the terminal-specific `PaneBinding` that connects a Workbench pane to a terminal key. The caller supplies terminal creation and resize functions, so this crate does not depend on the product event loop, App Server adapter, PTY implementation, or renderer.

`TerminalWorkspace<T, E>` retains terminal values of type `T` and buffers startup events of type `E`. `TerminalReady<T>` completes an earlier reservation, while `TerminalReadyOutcome<E>` tells the product whether the runtime became active, inactive, failed, or had already been removed.

Run `cargo test -p zeta-terminal-workspace` to verify reservation, activation, retry, binding, and removal semantics.
