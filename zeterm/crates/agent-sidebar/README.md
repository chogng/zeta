# `zeta-agent-sidebar`

`zeta-agent-sidebar` owns the retained product state for the Agent Sidebar's
Files and source-control-management (SCM) panes. It is the canonical
implementation-level contract for this surface; the cross-runtime product
architecture remains documented in [`docs/native-terminal-ui.md`](../../../docs/native-terminal-ui.md).

## Ownership

| Concern | Owner | Boundary |
| --- | --- | --- |
| Active Files/Changes pane and retained pane state | `AgentSidebar` | Does not know windows, app-server clients, or platform events. |
| Cross-pane selection and navigation | `AgentSidebarNavigation` | Owns only the Changes/Files switcher; it does not place Files or SCM controls. |
| Hierarchical entries, expansion, search query, and list scrolling | `files::FilesState` | Receives host-projected `DirectoryEntry` values, never protocol DTOs. |
| Files functional toolbar and tree/search geometry | `files::FilesLayout` / `files::FilesToolbar` / `files::FilesPane` | Owns Refresh, ahead/behind, Search, and the Files content bounds. |
| Changed-file snapshot | `scm::ScmState` | Receives host-projected `ScmDiff` values; Git transport stays outside the crate. |
| Changes functional layout and multi-diff presentation | `scm::ScmLayout` / `scm::EditorPane` | Owns the Changes content bounds and diff viewport interactions; delegates the nested inspection/interaction tree to `zeta-editor::MultiDiffEditor`. |
| Read-directory/open-file side effects | Native host | Executes `AgentSidebarAction` and supplies the resulting snapshot. |

The crate may depend on UI primitives, path search, editor document types, and
icons. It must not depend on `zeterm/zeterm`, app-server transport crates, a
window backend, or a Git transport implementation.

## Contract and execution path

`AgentSidebar::files_mut()` exposes the Files model to its presentation owner.
`FilesState::activate`, `navigate_right`, and `navigate_left` return an
`AgentSidebarAction`; `OpenFile` and `LoadChildren` are host obligations rather
than in-crate side effects. The host maps an authoritative directory response
to `DirectoryEntry` and calls `FilesState::refresh` or
`FilesState::complete_directory_load`.

```text
native event → FilesState interaction → AgentSidebarAction
             → native host executes file-system request
             → DirectoryEntry snapshot → FilesState
```

The outer sidebar supplies only the host rectangle and cross-pane navigation.
`FilesLayout`/`FilesToolbar` resolve the Files toolbar and content geometry;
`ScmLayout`/`EditorPane` resolve the Changes geometry. Neither feature imports
the other feature's layout or state.

`FilesTree` is private because stable node identity, loaded-child state, and
visible-row projection must change together. `FilesState::refresh` restarts the
path search index for the active root; a failed index start leaves the tree
usable but produces no search matches. Stale search snapshots are ignored by
their revision.

`ScmState` retains both the workspace-provided changed-file snapshot and the
`EditorPaneState` used by `EditorPane`. The host still owns the outer shell slot,
theme-token mapping, and execution of refresh/open actions; it must not retain a
second diff viewport beside `ScmState::editor`.

`EditorPane::compose` is the SCM composition boundary. It draws `MultiDiffEditor`
through `ComponentContext::draw_component` with the host identities for the
multi-diff root and scrollbar. `MultiDiffEditor` then owns the visible
`MultiDiffSection`, file header, `DiffEditor`, `CodeEditor`, fold-control, and
scrollbar components. The SCM host must not reconstruct child `UiNode`s; the
old `ComponentContext::register_node` bridge has been deleted.

## Modification guide

- Add Files interactions through `AgentSidebarAction`; do not call the
  filesystem or app-server directly from `files`.
- Extend source-control capability under `scm`; do not add SCM-specific state
  to `files` or generic UI primitives.
- Preserve `FilesTree` element IDs when an entry still represents the same
  path. Changing this invalidates focus and accessibility continuity.
- Preserve the changed-file state in `EditorPaneState` while the
  `MultiDiffEditor` component tree is rebuilt. `EditorPaneState` allocates a
  stable `MultiDiffEditorItemIdentity` per changed-file path, reuses the
  identity and `DiffEditorState` when snapshots reorder, and derives fold
  interaction IDs from that identity. The height animation key is exposed by
  `MultiDiffEditorItemIdentity`; transition binding and lifecycle cleanup remain
  zui runtime work.
- If a new host snapshot type is needed, add a crate-owned projection type and
  adapt it in the host; do not expose a protocol DTO in the public API.

## Verification

Run `cargo check -p zeta-agent-sidebar`,
`cargo test -p zeta-agent-sidebar`, and `cargo test --manifest-path Cargo.toml -p zeterm`. Files
model and pane presentation tests live in this crate; native tests cover only
the protocol adapter and shell composition.
