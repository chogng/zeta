# Text file service

This module owns the Workbench boundary between resource I/O and editor model
implementations. Cross-editor architecture and staged Alpha adoption are
canonical in [`docs/editor-architecture.md`](../../../../../../docs/editor-architecture.md).

## Current contract

| Concern | Owner | Status |
| --- | --- | --- |
| Workspace resource reads and atomic writes | `IFileService` | ✅ |
| Bootstrap-text versus file-system resolution | `ITextFileService.resolve` | ✅ |
| Text save transport and cancellation | `ITextFileService.save` | ✅ |
| URI-to-`TextModel` references | `ITextModelService` | ✅, editor-owned |
| Editor model references | Alpha and Gamma model services | ✅, editor-domain-owned |
| Format-specific dirty state, snapshot saves and explicit reverts | Alpha/Gamma model services | ✅, editor-domain-owned |
| Shared working-copy lifecycle and resource indexing | `IWorkingCopyService` | ✅, Workbench-owned contract, editor-owned implementation |
| CRLF/LF source-line-ending preservation | `ITextModelService` | ✅, editor-owned |
| Workspace external-change invalidation, clean reload, and dirty-model conflict state | `IFileService` → `ITextFileService` → `ITextModelService` | ✅, transport notification plus editor-owned policy |
| Pre-write external-change conflict detection | `ITextModelService` | ✅, editor-owned defense in depth |
| Atomic expected-revision writes and recovery | future TextFile model layer | 尚未完成 |
| Encoding and mixed line-ending preservation | future TextFile model layer | 尚未完成 |

`TextFileService.resolve` validates one `TextFileResolveRequest`, observes
cancellation, returns `bootstrapText` without touching the file system, and
otherwise delegates exactly one read to `IFileService.readFile`. The result
records whether its content came from `Bootstrap` or `FileSystem`.
`TextFileService.save` validates one `TextFileSaveRequest`, observes
cancellation, and delegates exactly one atomic write to `IFileService.writeFile`.
`ITextFileService.onDidChangeFiles` forwards coarse App Server filesystem
invalidations without introducing a live document cache. A concrete editor
decides whether a clean model may reload or a dirty model must retain local
text and report a conflict.

This service deliberately does not cache live models. Alpha and Gamma have
different transaction and undo semantics, so each editor domain owns its model
identity and reference lifetime. `IWorkingCopyService` indexes the resulting
format-specific working copies without owning their models. `ExplorerViewPane`
passes only a resource and label to `EditorPart`; the selected pane resolves
content through this service and registers its working copy with the shared
Workbench lifecycle.

## Ownership and failure semantics

`Workbench` constructs `TextFileService` after `BrowserFileService`, registers
it as `ITextFileService`, and injects it through `EditorPaneCreationOptions`.
Alpha and Gamma contributions reject construction when that service is absent.

Cancellation before resolution or save, or while awaiting the underlying I/O,
rejects without publishing a result. File-service errors pass through
unchanged. A non-text file-service result is rejected before it can enter an
editor model.

Adding model caches, backup recovery, or conflict policy directly to
`ExplorerViewPane` would signal architectural drift. Dirty state and conflict
policy remain in the editor-domain adapters (`BrowserTextModelService` for
Alpha and `DocumentWorkingCopy` for Gamma); the shared working-copy contract
exposes their common lifecycle without requiring a cross-editor document model.

## Tests and modification impact

`test/common/text-file-service.test.ts` covers bootstrap precedence, file
delegation, cancellation, validation, and failure propagation.
`../../../platform/files/test/browser/file-service.test.ts` covers App Server
invalidation projection.
`../../contrib/files/test/browser/explorer-view.test.ts` verifies that Explorer does not read file
content. Alpha model and pane tests cover shared model references, edit
preservation, cancellation, and session disposal. The working-copy service
test covers registration, lookup, and unregistration.

Changing resolution, save, or cancellation semantics requires updating all
three suites plus `docs/editor-architecture.md`. Expected-revision persistence
still requires separate conflict tests; it must not be represented as an
extension of the current transport-only result type.
