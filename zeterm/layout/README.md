# `zeta-layout`

`zeta-layout` owns zeterm's product-level pane topology while keeping the layout engine itself in
[`zui`](../zui/README.md). It resolves the root product/inspector split and the terminal workspace
with an optional right-hand sidebar. It is intentionally independent of `NativeApp`, commands,
terminal sessions, files, agents, renderers, and platform events.

## Ownership and execution path

Hosts project product state into `InspectorPane` and `SidebarLayoutSpec`, then call
`RootLayout::for_viewports` or `TerminalWorkspaceLayout::for_bounds`. The crate delegates sizing,
hidden-pane behavior, leaf lookup, and resize snapshots to `zui::{GridLayout, SplitViewPane}`.
The host uses the returned rectangles to compose domain views and routes the returned
`SplitViewResizeSnapshot` through its own pointer lifecycle.

| Symbol | Owner | Contract |
| --- | --- | --- |
| `LogicalViewport` | `zeta-layout` | Logical dimensions and physical-to-logical conversion |
| `InspectorPane` / `RootLayout` | `zeta-layout` | Product/inspector sibling topology |
| `SidebarVisibility` / `SidebarLayoutSpec` | host projection + `zeta-layout` | Host-neutral sidebar sizing policy |
| `TerminalWorkspaceLayout` | `zeta-layout` | Active workspace/sidebar bounds and resize geometry |
| `GridLayout`, `SplitViewPane` | [`zui`](../zui/README.md) | Generic layout algorithms and constraints |
| Session/agent/terminal state | product host/domain crates | State, persistence, events, and side effects |

## Modification impact

Changes to leaf topology, pane constraints, or resize geometry require updating the sibling tests in
`src/root_tests.rs` and `src/terminal_workspace_tests.rs`, then the zeterm host adapter in
`src/agent_sidebar.rs` and `src/shell_scene.rs`. If a change needs product state, command dispatch,
or renderer/platform APIs here, it belongs in the host or domain crate instead.

Verification:

```text
cargo test -p zeta-layout
```
