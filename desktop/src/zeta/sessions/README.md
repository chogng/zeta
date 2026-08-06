# Workbench sessions

`sessions` owns product-specific Workbench session profiles. A profile is the
initial composition of the shared Workbench for one product entry; it is not a
chat transcript, thread, or live `IWorkbenchSessionService` state.

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
