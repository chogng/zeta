# Text file service

This module owns the Workbench boundary between resource I/O and editor model
implementations. Cross-editor architecture and staged Alpha adoption are
canonical in [`docs/editor-architecture.md`](../../../../../../docs/editor-architecture.md).

## Current contract

| Concern | Owner | Status |
| --- | --- | --- |
| Workspace resource reads | `IFileService` | ✅ |
| Bootstrap-text versus file-system resolution | `ITextFileService.resolve` | ✅ |
| Alpha URI-to-`TextModel` references | `AlphaTextModelService` | ✅, editor-owned |
| Monaco URI-to-model references | `monacoModelService` | ✅, transition adapter-owned |
| Dirty state, save/revert and conflict resolution | future TextFile model layer | 尚未完成 |
| Encoding and external line-ending preservation | future TextFile model layer | 尚未完成 |

`TextFileService.resolve` validates one `TextFileResolveRequest`, observes
cancellation, returns `bootstrapText` without touching the file system, and
otherwise delegates exactly one read to `IFileService.readFile`. The result
records whether its content came from `Bootstrap` or `FileSystem`.

This service deliberately does not cache live models. Alpha and Monaco have
different transaction and undo semantics, so each editor domain owns its model
identity and reference lifetime. `ExplorerViewPane` passes only a resource and
label to `EditorPart`; the selected pane resolves content through this service.

## Ownership and failure semantics

`Workbench` constructs `TextFileService` after `BrowserFileService`, registers
it as `ITextFileService`, and injects it through `EditorPaneCreationOptions`.
Alpha, Monaco, and ProseMirror contributions reject construction when that
service is absent.

Cancellation before resolution or while awaiting the file read rejects without
publishing content. File-service errors pass through unchanged. A non-text
file-service result is rejected before it can enter an editor model.

Adding model caches, dirty flags, write APIs, backup recovery, or conflict
policy directly to `ExplorerViewPane`, `TextModel`, or a concrete editor pane
would signal architectural drift. Those capabilities require an explicit
TextFile model contract here and matching host write/durability support.

## Tests and modification impact

`desktop/test/text-file-service.test.ts` covers bootstrap precedence, file
delegation, cancellation, validation, and failure propagation.
`desktop/test/explorer-view.test.ts` verifies that Explorer does not read file
content. Alpha model and pane tests cover shared model references, edit
preservation, cancellation, and session disposal.

Changing resolution precedence or cancellation semantics requires updating all
three suites plus `docs/editor-architecture.md`. Adding persistence requires
separate save/revert/conflict tests; it must not be represented as an extension
of the read-only result type.
