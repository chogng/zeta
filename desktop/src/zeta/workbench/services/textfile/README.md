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
| Monaco URI-to-model references | `monacoModelService` | ✅, transition adapter-owned |
| Dirty state, snapshot saves and explicit reverts | `ITextModelService` | ✅, editor-owned |
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

This service deliberately does not cache live models. Alpha and Monaco have
different transaction and undo semantics, so each editor domain owns its model
identity and reference lifetime. `ExplorerViewPane` passes only a resource and
label to `EditorPart`; the selected pane resolves content through this service.

## Ownership and failure semantics

`Workbench` constructs `TextFileService` after `BrowserFileService`, registers
it as `ITextFileService`, and injects it through `EditorPaneCreationOptions`.
Alpha, Monaco, and ProseMirror contributions reject construction when that
service is absent.

Cancellation before resolution or save, or while awaiting the underlying I/O,
rejects without publishing a result. File-service errors pass through
unchanged. A non-text file-service result is rejected before it can enter an
editor model.

Adding model caches, dirty flags, backup recovery, or conflict policy directly
to `ExplorerViewPane`, `TextModel`, or a concrete editor pane would signal
architectural drift. Alpha currently keeps baseline comparison and its
transaction semantics in `BrowserTextModelService`; a cross-editor document
model is required before those semantics can move into this service.

## Tests and modification impact

`test/common/text-file-service.test.ts` covers bootstrap precedence, file
delegation, cancellation, validation, and failure propagation.
`../../../platform/files/test/browser/file-service.test.ts` covers App Server
invalidation projection.
`../../contrib/files/test/browser/explorer-view.test.ts` verifies that Explorer does not read file
content. Alpha model and pane tests cover shared model references, edit
preservation, cancellation, and session disposal.

Changing resolution, save, or cancellation semantics requires updating all
three suites plus `docs/editor-architecture.md`. Expected-revision persistence
still requires separate conflict tests; it must not be represented as an
extension of the current transport-only result type.
