# `zeta-workbench-controller`

`zeta-workbench-controller` is the product boundary between the reusable Workbench host and terminal-pane runtime identity. `WorkbenchController` owns one `zeta_workbench_host::WorkbenchHost<PaneBinding>` and exposes explicit access to the logical model and binding registry.

The logical Workbench remains the single owner of the one-to-one `TabInputKey` to `PaneContainer` relationship. The controller binds panes inside the selected container to product runtime identities without copying active-container state.

`PaneBinding` validates that a terminal key is attached only to a matching terminal input. Terminal processes, UI state, rendering, commands, and window events remain outside this crate.

Run `cargo test -p zeta-workbench-controller` to verify the binding contract.
