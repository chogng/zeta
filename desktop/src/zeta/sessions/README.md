# Workbench sessions

`sessions` owns product-specific Workbench session profiles. A profile is the
initial composition of the shared Workbench for one product entry; it is not a
chat transcript, thread, or live `IWorkbenchSessionService` state.

There are two deliberately separate session layers:

| Layer | Owner | Code / Academic boundary |
| --- | --- | --- |
| Workbench profile | `sessions/browser/*WorkbenchSession.ts` | separate initial layout and Composite selection |
| Runtime Chat Session | `workbench/services/sessions` + App Server | same protocol and kernel, separate product profile root |

Code and Academic may therefore share the Workbench and Rust App Server
implementation. They must not share the product runtime identity by accident.
`product/common/product.ts` declares the stable application ID, user-data
folder, and renderer storage namespace; Electron Main applies those values
before it creates persistent services. The App Server receives
`<product userData>/state` as `ZETA_PROFILE_ROOT`, so its SQLite session/thread
store and lease files are also isolated.

The current profiles are:

| Profile | Product entry | Editor bundle | Default layout |
| --- | --- | --- | --- |
| `code` | `workbench-code` | `editor/alpha/editor.all` | Explorer + Terminal panel, Chat/Auxiliary Bar visible |
| `academic` | `workbench-academic` | `editor/gama/editor.all` | wider Sidebar, Problems panel, document-first central surface, Auxiliary Bar hidden |
| `complete` | `workbench-complete` | Alpha + Gama `editor.all` | combined Code + Academic composition with Terminal panel |

`createWorkbenchSession` validates and freezes the profile before it crosses
the product-to-Workbench boundary. Each product entry composes exactly one
profile with its declared editor public bundle; the shared Workbench consumes
only the generic `WorkbenchSession` contract, applies its region layout and
initial Composite selection, and does not import these product profiles or any
product contribution.

The profile is a default, not a forced reset. `WorkbenchLayoutStateModel`
loads the stored workspace layout after the profile is selected, so users keep
their manual resizing and visibility changes within the product/workspace
storage namespace. `SessionsPart` remains an optional runtime status Part and
does not own layout topology.

## Same-machine installation

Installing the Code and Academic editions together is supported when the
installer consumes each product's `applicationId` and the runtime keeps each
product's `userDataFolderName` and `sessionData` distinct. A shared kernel is
safe because it is code and protocol, not a shared mutable state directory.

Opening the same workspace in both editions is still a shared-file scenario:
file edits, external changes, and workspace-level tools can observe the same
workspace. Product isolation does not turn one workspace into two copies.
