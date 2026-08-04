# Alpha Editor

> 文件级架构、VS Code editor 对照表和全量 `contrib` 迁移清单见 [`editor-architecture.md`](./editor-architecture.md)。本文记录实现契约、当前行为、测试证据和已知限制。

> 按 feature 逐文件阅读当前实现见 [`alpha-implementation-ledger.md`](./alpha-implementation-ledger.md)。

> 本文拥有文本内核的实现和修改契约。跨 editor、document、language、
> browser view、Workbench 与退场中的 Monaco adapter 边界见
> [`docs/editor-architecture.md`](../../../../../docs/editor-architecture.md)。
> Rust document-core 的跨运行时分层与迁移阶段见
> [`docs/editor-core.md`](../../../../../docs/editor-core.md)。

Alpha owns the editor-domain primitives and is the product's default plain-text
editor. Its common text model remains independent of Monaco, ProseMirror, the DOM,
workbench tabs, files, persistence, and external language runtimes; browser composition
consumes platform capabilities only through named service contracts. Monaco is being
retired, receives no new product capability, and must not define the canonical Zeta
text-model contract.

## Runtime layering and base dependencies

| Directory | May depend on | Owns |
| --- | --- | --- |
| `common/` | `base/common` | DOM-free editor identities, text, history, selection, decoration, composition transactions, and pure layout/view-model math |
| `browser/` | `base/common`, `base/browser`, `alpha/common`, `alpha/contrib` | DOM projection, textarea/input events, viewport observation, font measurement, accessibility, and browser adapters |
| `contrib/folding/browser/` | `base/common`, `base/browser`, `alpha/common`, `alpha/browser/view` | Code-folding range providers, tracked fold state, fold commands, gutter presentation, and folded visual-row projection |
| `common/languages` + `common/tokens` | `base/common`, `alpha/common/core`, `alpha/common/model` | Alpha language configuration, provider contracts, analysis/completion wire, versioned results, token index and lexical editing; no DOM or Workbench |
| `../monaco/`, `../prosemirror/` | their adapter dependencies plus stable Alpha contracts | Retirement/migration integration only; no new product ownership |
| Workbench contributions | editor public contracts and platform services | Pane registration, product composition, and document/workspace wiring |

The dependency direction is intentionally one-way: editor layers should reuse
base primitives, while `base` must never import or specialize for editor
concepts. Current common code already consumes `base/common/event`,
`base/common/lifecycle`, and the realm-wide `base/common/ime` coordination
state. Viewport geometry reuses `base/common/layout.ISize`; browser input should
reuse base browser DOM/lifecycle helpers and platform keybinding state.
Text positions, model versions, selections, decorations, viewport semantics,
and editor instance identity remain editor-owned even when implemented on base
primitives.

Language capability is deliberately outside Alpha's core ownership. The
`ILanguageFeaturesService` contract in
`common/services/languageService.ts` (Workbench's
`workbench/services/language/common/languageFeaturesService.ts` is only a DI wrapper)
owns shared language registrations and creates caller-owned per-document analysis
and completion services. `AlphaEditorSession` may use the default implementation
or receive a host-provided service; it does not own a language runtime, grammar,
or provider registry. This is the replacement seam for an extension host, LSP, or
Rust-backed provider.

### Service extraction boundary

Service extraction follows a real cross-module owner, not the size of an Alpha
class:

| Concern | Owner | Current decision |
| --- | --- | --- |
| Events, lifecycle, URI identity, resource collections | `base/common` | Reuse existing primitives; editor semantics must not flow back into base |
| Raw resource I/O | `platform/files` | `IFileService` owns workspace reads and App-Server-backed atomic UTF-8 writes |
| Text resource loading and save transport | `workbench/services/textfile/common` | ✅ `ITextFileService.resolve/save`; it deliberately does not own a live editor model |
| Syntax token production | `workbench/services/textMate` and Alpha analysis providers | ✅ Bundled language packages are declarative resources loaded through the Workbench TextMate service; Rust owns discovery/resource transport and Alpha owns tokenization |
| Shared URI-to-Alpha-model references, saved baseline and dirty state | `ITextModelService` | ✅ editor-owned; Alpha owns LF-normalized baseline comparison, serialized snapshot saves and explicit reverts |
| Text transactions, history, selections, decorations and versioned language results | `editor/alpha/common` | ✅ Alpha's synchronous TypeScript authority; no Rust/WASM shadow document |
| Rust file/language/workspace capability | frontend domain services over App Server | Asynchronous, revision-bound results only; never the keystroke hot path |
| Language identities and composable editing rules | `editor/alpha/common` | `LanguageConfigurationRegistry`; comments/brackets/pairs are editor-domain contracts, not generic base primitives |
| TextMate grammar loading and token production | `workbench/services/textMate` adapter over Alpha analysis providers | 部分具备; bundled language grammars, file associations, language configuration, snippets, static discovery, and serializable scope-theme rules are wired; theme activation and full embedded-language/bracket-result projection remain open |

The current `textfile` contract was extracted only after Alpha, Monaco,
ProseMirror, and Explorer established a real shared loading boundary. It now
owns transport-only resolve/save operations; Alpha's baseline, dirty state and
transaction semantics remain editor-owned. `fs/changed` invalidation is
forwarded through this boundary: Alpha reloads a clean shared model and marks
a dirty model externally changed while retaining its local edits.
Expected-revision writes remain a future document-layer contract.
The TextMate service was extracted only after Alpha's Analysis provider/module
path became its real consumer. Conversely, file loading or grammar-runtime imports appearing inside
`TextModel`, `LanguageTokenLineIndex`, or Alpha browser components are an
architectural drift signal and require extraction at that point.

## Current implementation

| Capability | Status | Canonical owner |
| --- | --- | --- |
| Zero-based text positions and end-exclusive ranges | ✅ | `common/core/{position,range}.ts` |
| VS Code-shaped DOM-free editor core algebra | ✅ | `common/core/{ranges,text,edits,misc,2d}` |
| LF-normalized text and line lookup | ✅ | `TextModel` |
| Allocation-free document and line lengths | ✅ | `TextModel.length` / `getLineLength` |
| Atomic non-overlapping edit transactions | ✅ | `TextModel.applyEdits` |
| Non-undoable document reset for reload/revert | ✅ | `TextModel.reset` / `TextModelChangeReason.Reset` |
| Monotonic versions and synchronous change events | ✅ | `TextModel.commitOffsetEdits` |
| Transaction-level undo and redo | ✅ | `TextModel.undo` / `redo` |
| Piece-tree storage and incremental line counts | ✅ | `PieceTreeTextBuffer` |
| Adjacent piece coalescing | ✅ | `PieceTreeTextBuffer.mergeCoalescing` |
| Immutable versioned snapshots | ✅ | `TextModel.createSnapshot` |
| Transaction and text-unit history limits | ✅ | `TextModel` history policy |
| Snapshot-safe buffer compaction | ✅ | `PieceTreeTextBuffer.compactIfNeeded` / browser-owned `TextModel.maintenance` |
| Repeatable storage benchmark | ✅ | `benchmark/pieceTreeTextBuffer.benchmark.ts` |
| Tracked storage performance budgets | ✅, opt-in CI gate | `benchmark/pieceTreeTextBuffer.benchmark.ts` |
| Literal/regex document search, Unicode whole-word and wrap-next | ✅ | `common/model/textModelSearch.ts` |
| Capture-aware replacement commands and isolated undo | ✅ | `common/commands/textSearchCommands.ts` |
| Find/replace widget, selection scope, shortcuts, navigation and match decoration | ✅ | `browser/AlphaFindController` |
| Go to line/column/offset input and preview | ✅ | `common/commands/gotoLocation.ts` / `AlphaGotoLineController` |
| Direction-aware selection and multi-selection values | ✅ | `common/core/selection.ts` |
| Alt+Shift rectangular column selection | ✅ | `common/cursor/columnSelection.ts` / `AlphaPointerSelectionController` |
| Unicode-safe word-selection segments | ✅ | `common/cursor/wordBoundary.ts` |
| Shared grapheme and word segmentation | ✅ | `common/core/textSegmentation.ts` |
| Multi-selection cursor navigation commands | ✅ | `common/cursor/cursorNavigation.ts` |
| Multi-selection type, Backspace, and Delete commands | ✅ | `common/cursor/cursorTypeOperations.ts` / `common/cursor/cursorDeleteOperations.ts` |
| Grapheme-safe transient overtype input | ✅ | `common/cursor/cursorOvertype.ts` / `AlphaTextInputController` |
| Shared-boundary word deletion commands | ✅ | `common/cursor/cursorWordOperations.ts` / `AlphaTextInputController` |
| Browser soft-line deletion to start/end | ✅ | `common/cursor/cursorDeleteOperations.ts` / `AlphaTextInputController` |
| Multi-selection line indent and outdent | ✅ | `contrib/linesOperations/browser/lineIndentCommands.ts` |
| Language-aware toggle line comment | ✅ | `common/commands/lineCommentCommands.ts` / `AlphaLineCommentController` |
| Language-aware toggle block comment | ✅ | `common/commands/blockCommentCommands.ts` / `AlphaBlockCommentController` |
| Insert, delete, duplicate, and move line groups | ✅ | `contrib/linesOperations/browser/{linesOperations,lineOperationsController}` |
| Join selected lines with whitespace normalization | ✅ | `common/commands/lineJoin.ts` / `AlphaLineJoinController` |
| Grapheme-safe character transpose | ✅ | `common/cursor/cursorTranspose.ts` / `AlphaTransposeController` |
| Add/Select exact text occurrences | ✅ | `contrib/multicursor/common/occurrenceSelection.ts` / `AlphaOccurrenceSelectionController` |
| Current word/selection occurrence highlight | ✅ | `contrib/wordHighlighter/common/wordHighlighter.ts` / `AlphaOccurrenceHighlightController` |
| Platform-aware adjacent and selected-line-end cursors | ✅ | `common/cursor/cursorInsertion.ts` / `AlphaMultiCursorController` |
| Cursor-only undo after multi-cursor and occurrence operations | ✅ | `common/cursor/editorSelectionController.ts` / `AlphaCursorUndoController` |
| Repeated physical-line selection expansion | ✅ | `contrib/lineSelection/browser/lineSelection.ts` / `AlphaEditingCommandController` |
| Lexically filtered current bracket-pair highlighting | ✅ | `LanguageBracketMatcher` / `AlphaBracketMatchController` |
| Lexical nested bracket colorization | ✅ | `LanguageBracketColorizationIndex` / `AlphaBracketColorizationSource` |
| Go to/select configured matching brackets | ✅ | `contrib/bracketMatching/common/bracketNavigation.ts` / `AlphaBracketNavigationController` |
| Remove configured matching bracket pairs | ✅ | `contrib/bracketMatching/common/bracketEditing.ts` / `AlphaBracketEditingController` |
| Select-all and line-indentation shortcuts | ✅ | `browser/AlphaEditingCommandController` / `contrib/linesOperations/browser/AlphaLineOperationsController` |
| Transaction-aware tracked ranges | ✅ | `TextModel.trackRange` |
| Per-editor live selection state | ✅ | `EditorSelectionController` |
| Selection-aware command undo and redo | ✅ | `EditorSelectionController.execute` |
| Adjacent single/multi-cursor typing and deletion coalescing | ✅ | `TextEditHistoryGroup` |
| Single-selection IME composition transaction lifecycle | ✅ | `EditorCompositionSession` |
| Domain-neutral decoration collections | ✅ | `TextDecorationCollection` |
| Named browser decoration presentation | ✅ | `createAlphaDecorationSource` / `AlphaEditorViewport` |
| Versioned cancellable language request gate | ✅ | `LanguageRequestCoordinator` |
| Versioned token and diagnostic result stores | ✅ | `VersionedLanguageResultStore` / `languageResults.ts` |
| Versioned diagnostic decoration bridge | ✅ | `LanguageDiagnosticDecorationBridge` |
| Diagnostic browser presentation | ✅ | Error/Warning/Information/Hint underlines, rich gutter hover, overview ruler, and highest-severity gutter marker |
| Next/previous diagnostic navigation and live announcement | ✅ | `AlphaDiagnosticNavigationController` (F8 / Shift+F8) |
| Semantic-token line index, closed modifier styling, and browser projection | ✅ | `LanguageTokenLineIndex` / `createAlphaSemanticTokenSource` |
| Confirmed-delta token line reuse and visible-line resolution | ✅ | `LanguageTokenLineIndex` / `AlphaSemanticTokenSource.getLineTokens` |
| Multi-splice analysis delta and relative suffix payload reuse | ✅ | `languageAnalysisItemDelta.ts` / `LanguageTokenLineIndex` |
| Shared token/diagnostic Worker transport | ✅ | `LanguageAnalysisService` / `languageAnalysisWireCodec` / `createAnalysisWorkerFactory` |
| Incremental analysis result transport | ✅ | `languageAnalysisWireCodec` / wire protocol v4 |
| Baseline lexical tokens and structural diagnostics | ✅ | `createLanguageLexicalAnalysisProvider` |
| Composable language editing configuration | ✅ | `LanguageConfigurationRegistry` |
| Versioned language-isolated lexical line cache | ✅ | `LanguageLexicalAnalysisCache` / `LanguageLexicalLineScanner` |
| Shared language provider-module lifecycle | ✅ | `LanguageProviderModuleRegistry` / `LanguageProviderModuleHost` / generic module wire |
| Analysis provider-module activation barrier | ✅ | `LanguageAnalysisModuleWorkerClient` / `language.lexical` |
| TextMate grammar provider and Worker seam | 部分具备 | `workbench/services/textMate`; JSON/JSONC extension resources, explicit session grammar contributions, catalog transport and Alpha pane Worker selection are active |
| Versioned completion result, session, and widget | ✅ | `languageCompletions.ts` / `LanguageCompletionSessionController` / `AlphaCompletionWidget` |
| Completion snippet tabstops, variables, choices, and navigation-refreshed regex transforms | ✅ | `languageCompletionSnippet.ts` / `languageCompletionSnippetTransform.ts` |
| Completion provider registry, host, and input triggers | ✅ | `LanguageCompletionProviderRegistry` / `LanguageCompletionService` / `AlphaTextInputController` |
| Completion Worker wire and lexical provider | ✅ | `LanguageWorkerWireClient` / `languageCompletionWireCodec` / `createCompletionWorkerFactory` / `language.word` |
| Incremental Worker document mirror | ✅ | `LanguageWorkerModelSynchronizer` / `LanguageWorkerDocumentMirror` / wire protocol v4 |
| Worker provider metadata catalog | ✅ | `LanguageCompletionProviderCatalog` / `LanguageCompletionCatalogWorkerClient` / `LanguageCompletionCatalogWirePublisher` |
| Named Worker provider-module activation | ✅ | `LanguageCompletionProviderModuleRegistry` / `LanguageCompletionProviderModuleHost` / module wire protocol |
| Deferred completion details and resolve wire | ✅ | `LanguageCompletionItemResolver` / `LanguageCompletionResolveWireClient` / session details state |
| Per-editor auto-closing provenance | ✅ | `contrib/bracketMatching/common/autoClosingTracker.ts` / `pairEditing.ts` |
| Fixed-height visual-row viewport, scrolling, and overscan | ✅ | `EditorViewportModel` / `EditorViewportLineSource` |
| Measured grapheme-safe soft wrapping and visual-row mapping | ✅ | `AlphaVisualLineProjection` / `EditorVisualLineProjection` |
| Bounded document minimap, GPU density projection with DOM fallback, click/drag scroll navigation | ✅ | `minimapProjection.ts` / `AlphaGpuMinimapRenderer` / `AlphaMinimapNavigationController` / `AlphaEditorViewport` |
| Version-pinned, cancellable diff model with Rust-backed computation and grapheme-safe inline ranges | ✅ | `common/diff/DiffModel` / `IDiffComputationService` / `browser/services/rustDiffComputationService.ts` |
| Virtualized side-by-side review view | ✅ | `browser/widget/diffEditor/DiffEditorWidget` |
| Workbench original/modified diff input and pane lifecycle | ✅ | `AlphaDiffEditorInput` / `AlphaDiffEditorPane` |
| Visible-line indentation guides | ✅ | `contrib/indentation/browser/indentation.ts` / `AlphaEditorViewport` |
| Indentation/lexical/named-region/manual folding, visible-row projection, gutter toggle, recursive/level fold chords, collapse-all and expand-all | ✅ | `contrib/folding/browser/{foldingRanges,syntaxRangeProvider,indentRangeProvider,foldingModel,hiddenRangeModel,foldingDecorations,folding}` plus reusable `browser/{visibleLineProjection,visualLineProjection}`; language markers come from `LanguageConfiguration.foldingMarkers` |
| Transient Alt+Z word-wrap toggle | ✅ | `AlphaEditorViewport` / `AlphaWordWrapController` |
| Read-only virtual line DOM projection | ✅ | `browser/AlphaEditorViewport` |
| Canonical browser CodeEditor composition | ✅ | `browser/widget/codeEditor/CodeEditorWidget`; owns viewport, native input, keyboard/pointer navigation, and text drop while model and selections remain caller-owned |
| Computed-font measurement and incremental line widths | ✅ | `AlphaDomTextMeasurer` / `AlphaLineWidthIndex` |
| Virtual line-number gutter | ✅ | `browser/AlphaEditorViewport` |
| Multi-selection and caret geometry/DOM projection | ✅ | `createAlphaSelectionGeometry` / `AlphaEditorViewport` |
| Focused caret blinking with reduced-motion fallback | ✅ | `media/alphaEditorViewport.css` |
| Pointer client-coordinate hit testing | ✅ | `hitTestAlphaEditorPoint` / `getTargetAtClientPoint` |
| Pointer character/word/line click and drag selection | ✅ | `AlphaPointerSelectionController` |
| Context-menu selection preservation | ✅ | `AlphaPointerSelectionController` |
| Pointer drag autoscroll | ✅ | `AlphaPointerAutoScroller` |
| Configurable modifier-based pointer multi-cursor | ✅ | `pointerMultiCursor.ts` |
| Platform-aware physical/visual-row keyboard navigation and reveal | ✅ | `AlphaKeyboardNavigationController` / `navigateAlphaVisualCursors` |
| Hidden textarea ordinary text editing and history | ✅ | `AlphaTextInputController` |
| Per-input read-only edit gate with selection/navigation retained | ✅ | `EditorSelectionController` / `EditorInput.readOnly` |
| Language-aware auto-close, surround, overtype, and paired Backspace | ✅ | `contrib/bracketMatching/common/pairEditing.ts` / `AlphaTextInputControllerOptions.language` |
| Language-aware Enter and editor-owned indentation | ✅ | `contrib/bracketMatching/common/enter.ts` / `contrib/indentation/common/indentation.ts` |
| Synchronous lexical input context and auto-closing `notIn` | ✅ | `LanguageLexicalContextIndex` / `LanguageLexicalContextSource` |
| Plain-text and syntax-marked safe HTML selection copy, cut, and paste, including Async Clipboard fallback | ✅ | `AlphaClipboardController` / `clipboardRichText.ts` / `syntaxClipboardHtml.ts` |
| User-supplied single-text-file clipboard paste | ✅ | `AlphaClipboardController` / `textFileTransfer.ts` |
| Local declared MIME paste providers and `text/uri-list` paste | ✅ | `clipboardPasteProvider.ts` / `AlphaClipboardController` |
| Empty-selection whole-line copy, cut, and paste | ✅ | `common/commands/clipboard.ts` / `AlphaClipboardController` |
| Plain-text and user-supplied single-text-file drop at the viewport hit target | ✅ | `AlphaTextDropController` / `textFileTransfer.ts` |
| Desktop-style textarea composition and candidate anchor | ✅ | `AlphaCompositionController` |
| Tracked multi-line composition underline | ✅ | `EditorCompositionSession.currentRange` / `AlphaEditorViewport` |
| Focused accessible text/primary-selection mirror, multi-selection description, status announcements, and forced-colors semantics | ✅ | `AlphaTextInputController` / `AlphaEditorViewport` / `AlphaSaveController` |
| macOS desktop IME/VoiceOver DOM contract | ✅ | `AlphaCompositionController` / `AlphaTextInputController`; empirical VoiceOver walkthrough remains a release verification task |
| Mobile IME variants and cross-platform assistive-technology acceptance | 不在 Alpha 桌面完成范围 | Mobile is explicitly out of scope; Windows validation is separate release verification |
| File and bootstrap loading | ✅ | `ITextFileService` / `ITextModelService` |
| Dirty state and serialized snapshot saving | ✅ | `ITextModelService` / `AlphaSaveController` |
| Explicit file-system revert | ✅ | `TextModelReference.revert` / `AlphaEditorPane.revert` |
| CRLF/LF source line-ending preservation on save | ✅ | `ITextModelService` |
| Workspace external-change invalidation, clean reload, and dirty-model conflict state | ✅ | `IFileService.onDidChangeFiles` / `ITextModelService` |
| Pre-write external-change conflict detection | ✅, defense in depth | `ITextModelService` |
| Atomic expected-revision writes and backup recovery | 尚未完成 | Future document-layer contract |

`TextPosition` names both indices explicitly and counts UTF-16 code units so
conversion to JavaScript strings and browser selections is deterministic.
`TextRange` is ordered and end-exclusive. Input text normalizes CRLF, CR,
Unicode line separator, and paragraph separator to LF before it enters the
model. Alpha records whether loaded or reverted content uses CRLF and restores
that convention on save; all other input is written as LF. Mixed line endings
and encoding policy remain document-layer concerns. Coarse workspace change
events reload clean Alpha models; dirty models retain local content, expose
`hasExternalChange`, announce the conflict to assistive technology, and keep
the save-time baseline check as a second defense.

When focused, `AlphaTextInputController` mirrors the normalized model into its
native textarea with the primary selection's offsets and direction. Native
selection changes then update Alpha's primary selection if they truly differ;
ordinary programmatic synchronization leaves multi-cursor state intact. The
mirror is intentionally suspended during IME composition and released on blur.

Each `applyEdits` call is one transaction. All ranges refer to the document
before that transaction, input ordering has no effect, and overlap is rejected
before mutation. Successful edit, undo, and redo operations each increment the
version and synchronously emit one immutable `TextModelChange`.

`createSnapshot` captures immutable source segments without eagerly joining the
whole document. The snapshot records the model version, length, and line count,
supports offset-range reads, and remains valid after later edits or model
disposal.

History defaults to at most 1,000 transactions and 16,777,216 stored UTF-16
text units. Callers with a different workload can state both limits explicitly:

```ts
const model = new TextModel(initialText, {
  historyLimit: {
    transactions: 200,
    textUnits: 4 * 1_024 * 1_024,
  },
});
const snapshot = model.createSnapshot();
```

After each committed transaction, storage compacts when it exceeds 4,096
pieces, retains at least twice the live document with at least 65,536
reclaimable text units, or retains at least 67,108,864 reclaimable units.
Compaction joins the current text into a new immutable original buffer and
resets the add buffer. It does not increment the version, emit an event, or
change history. Existing snapshots keep references to their captured source
strings and remain readable.

`findTextMatches` scans one immutable model snapshot and returns model-versioned,
UTF-16 ranges with numbered and named regular-expression captures. Literal and
regular-expression modes share case and Unicode whole-word policy; invalid
expressions fail before projection. `textSearchCommands.ts` rejects stale
matches, expands regular-expression replacement references, and creates single
or replace-all edits as isolated selection-aware undo transactions.

`AlphaFindController` owns only the browser dialog, Ctrl/Cmd+F and replace
shortcuts, match navigation, and the caller-owned search-decoration projection.
It captures a non-empty opening selection as a `NeverGrowsAtEdges` tracked
scope. The Find in Selection control and Alt+L apply that same scope to both
navigation and replace-all, even after navigation changes the visible editor
selection. The model and edit semantics stay in `common/`; the viewport only
renders the named `SearchMatch` presentation. Its component DOM states project
`.visible` and `.checked` alongside ARIA state, and `media/findWidget.css` owns
their visual rules. Search remains bounded to 999 projected matches while
replace-all accepts up to the common search limit of 100,000 matches in one
transaction.

`LanguageRequestCoordinator` captures one of these snapshots per request. Work
in the same named lane is latest-wins, while independent lanes may share one
lazily created worker.
Model changes, caller abort, worker restart, and disposal all produce explicit
cancelled outcomes. A worker value reaches its synchronous application callback
only while both request identity and `TextModel.version` still match. The
coordinator owns its worker but not the model; an active worker failure disposes
that instance and the next request creates a replacement.

`VersionedLanguageResultStore` is the persistent side of that application
boundary. `VersionedLanguageResult` carries the exact realm-local `TextModel`
identity in addition to its version, so two documents at version 1 cannot be
cross-wired. The store accepts one monotonic request-ID domain, retains a
request high-water mark across explicit clears, and emits immutable `Result`,
`ModelChanged`, or `Cleared` events. A model
transaction clears language state instead of mapping obsolete token or
diagnostic ranges through the edit. Generic normalizers must return immutable
values; `createLanguageTokenStore` and `createLanguageDiagnosticStore` provide
strict canonical normalizers. Tokens are non-empty, single-line, sorted,
non-overlapping spans with unique modifiers. Diagnostics may overlap or use a
point range and carry validated severity, message, code, and source metadata.

`LanguageTokenLineIndex` groups the current immutable token result into sparse
line buckets and exposes constant-time visible-line queries. Model invalidation
immediately hides all buckets but may retain their immutable state as a private
confirmed delta base. A matching multi-splice result rebuilds only changed
sparse lines. Each line keeps immutable line-relative token payloads; unchanged
lines with stable indices reuse their public line object, while shifted suffix
lines reuse the payload and lazily materialize absolute `TextRange` values only
when queried. It owns neither the store nor model.
The browser source maps token types through a closed Alpha presentation enum;
unknown worker strings are omitted rather than becoming CSS classes. Line DOM
is segmented exclusively with text nodes and named spans, with exact source
text verified before replacement. Semantic colors are registered through the
platform theme registry rather than defined in core or raw CSS.

`LanguageAnalysisService` owns token and diagnostic result stores plus one
multi-lane request coordinator. Token and diagnostic requests are independently
latest-wins and may run concurrently over one Worker/document mirror. The first
ordered request initializes the mirror with a full snapshot; the peer lane and
later requests reference it, while model transactions send one incremental
sync shared by both lanes.

Browser sessions create their analysis worker directly from the editor-local provider factory.
Bundled language packages use declarative TextMate grammars where a grammar is present; Alpha's
deterministic lexical provider remains the editor-owned baseline for unsupported or unavailable
grammar roots. Workbench, Renderer API and App Server do not expose a syntax service. Rust
macro-aware tokens and parser-grade syntax require a future provider behind the same Alpha
provider/Worker boundary; parser transport remains private to the editor implementation. Monaco has
no ownership in this path and receives no parallel syntax integration.

`LanguageAnalysisProviderRegistry` selects the first matching token provider
and all matching diagnostic providers in registration order. Provider failures
are reported and isolated: a failed token provider yields an empty token batch,
while healthy diagnostic batches still merge. Both provider output and remote
DTOs are normalized against the captured snapshot before they can reach the
versioned stores.

`createAnalysisWorkerFactory` owns a real Vite module Worker using the
strict multi-lane `languageAnalysisWireCodec`. Its Worker entry publishes an
`language.lexical` provider module, while `LanguageAnalysisModuleWorkerClient`
waits for catalog discovery and activation before releasing the first analysis
request. The module loads a deterministic TypeScript/JavaScript/JSON baseline
for comments, strings, numbers, identifiers, keywords, operators, unterminated
literals, and bracket balance.

Completion and analysis share `LanguageProviderModuleRegistry`,
`LanguageProviderModuleHost`, catalog normalization, required-module
activation, and the generic module wire state machine. These contracts remain
in `alpha/common`: they encode language-provider ownership and therefore do
not belong in domain-neutral `base`. Their Completion and Analysis files are
typed façades that retain protocol and provider boundaries. Waiting for a
promise with a caller-owned `AbortSignal` is genuinely cross-domain, so that
mechanism is reused as `base/common/cancellation.raceCancellation`.

`LanguageWorkerWireServer` calls a
`LanguageWorkerDocumentSynchronizationObserver` only after its Piece Tree
mirror has atomically accepted an ordered transaction. The analysis provider
host forwards the new immutable snapshot to registered document synchronizers;
one synchronizer failure is reported as `synchronization` and does not block
healthy request lanes.

`LanguageConfigurationRegistry` composes immutable comments and bracket rules
field by field. Higher priority wins, later registration breaks equal-priority
ties, `null` explicitly clears an inherited field, and disposing a contribution
restores the remaining configuration. Language IDs and the `*` provider
selector share one editor-owned validation contract; neither belongs in
domain-neutral `base`.

The lexical provider owns a cache per language/configuration identity, and each
cache is shared by both lexical lanes. This prevents a TypeScript result from
being reused when the same model version is requested as JSON. JSON, JSONC,
and ECMAScript now receive distinct comment/string profiles; changing a
registry contribution replaces same-version lexical state.
`LanguageLexicalLineScanner` returns relative token spans, ordered
diagnostic/bracket/multiline events, and explicit input/output lexical state.
`updateLines` reuses an exact line prefix, scans from the first changed line,
and reuses the remaining exact suffix as soon as its cached input state equals
the newly propagated state. `aggregateResults` binds relative data to the
current line indices and builds both immutable result batches once per model
version. Bypassing this cache from either provider method, storing absolute
line indices in a line result, or teaching the generic Worker mirror lexical
rules would be architectural drift.

The optional cache update observer exposes full/incremental mode plus scanned
and reused line counts for tests and profiling; it is not a renderer state
channel. The implementation is intentionally lexical rather than an AST,
type checker, or language-server analysis graph.

Wire protocol v4 retains the per-lane result baseline without assuming that a sent
response was received. A codec declares either the `stateless` or
`confirmedBase` result protocol; completion uses the former and analysis the
latter. A decoded confirmed-base result is staged until
`LanguageRequestCoordinator` reports `Applied` after the renderer application
callback. Cancelled, stale, or application-failed results are discarded.
`LanguageWorkerWireClient` includes only its last applied request ID in
`resultBaseRequestId`.
`LanguageWorkerWireServer` supplies a prior result to the codec only when its
last state has that exact ID. A missing response, cancellation race, stale
request, or replaced Worker therefore produces a full result instead of an
unusable delta.

`createLanguageAnalysisItemSplices` compares exact snapshot lines and normalized
items, retains ordered stable runs, and emits disjoint splices in base-item
coordinates. Each splice carries the cumulative line shift after its changed
region, allowing unchanged middle and suffix runs to move independently.
`encodeLanguageAnalysisWireResult` binds the splice list to `baseRequestId`;
`decodeLanguageAnalysisWireResult` checks ordering, overlap, base bounds, final
line shift, lane, final ranges, token ordering, and diagnostic metadata before
publishing a reconstructed realm-local result. A delta is used only when it
transfers fewer items than a full result.

The token codec attaches validated ordered-splice metadata through a realm-local
`WeakMap`; provider normalization strips it, while the token store preserves
only metadata created after DTO validation. `LanguageTokenLineIndex` maps each
unchanged base-item segment into the final result, reuses relative line
payloads across piecewise line shifts, reports rebuilt/reused sparse-line
counts, and falls back to a complete index build when the base is absent or any
mapping invariant fails.

`AlphaSemanticTokenSource.getLineTokens` resolves one requested line through
the closed presentation vocabulary. `AlphaEditorViewport` resolves the target
virtual window before mutating its rows, preserving window-level failure
atomicity without resolving offscreen token lines.

`VersionedLanguageResultStore` still publishes a complete absolute token array,
and moved suffixes still allocate lightweight line bindings when their public
line indices change. The index no longer rebuilds their semantic payloads or
absolute token ranges eagerly. A persistent result tree that avoids rebuilding
the complete renderer token array remains future work.

`createLanguageCompletionStore` validates immutable completion results at the
same version gate. Every item has a provider/item composite identity, named kind, explicit
single-line replacement range, and insertion text; the range must contain the
result's trigger position. `LanguageCompletionSessionController` owns only
per-editor focus. It opens for one matching collapsed caret, retains focus by
item identity across same-version refreshes, and accepts through one isolated
`EditorEditCommand`. Label and detail text never define the edit.

`AlphaCompletionWidget` projects that session below the measured caret and is
optionally owned by `AlphaTextInputController`. It coordinates listbox ARIA,
stable `.visible`/`.focused` state classes, cyclic keyboard focus, Enter/Tab,
Escape, and mouse acceptance without taking ownership of common language or
selection state.

`LanguageCompletionProviderRegistry` preserves registration order and selects
providers by exact language ID or `*`, with optional trigger characters.
`LanguageCompletionProviderWorker` runs matching providers concurrently behind
the existing `LanguageWorker` contract, validates every batch against one shared
captured-snapshot line index, isolates provider failures, and merges by
registration/item order. Cancellation is not converted into a provider error.
The first preselection wins and incompleteness is combined across providers.

`LanguageCompletionService` owns the request coordinator, provider hosts, and
completion result store, but not the model or registry. The browser request
adapter sends explicit Invoke, TriggerCharacter, or IncompleteRefresh contexts.
Ctrl+Space invokes manually; character-triggered and incomplete-refresh requests
run only after the text transaction has published its new model version and
caret. A session must observe the exact service-owned result store, not merely a
store associated with the same model.

`LanguageWorkerWireClient` and `LanguageWorkerWireServer` provide a DOM-free,
versioned request/cancel/result/failure protocol over structural message ports.
The initial request carries a complete immutable LF snapshot DTO. The server verifies
version, UTF-16 length, and line count before rebuilding its snapshot; the
completion codec reconstructs realm-local `TextPosition` and `TextRange`
instances in the receiving realm and repeats snapshot-bound normalization.
Cancellation aborts remote work, terminal transport failure rejects all pending
requests, and the existing coordinator replaces the client on the next request.

`createCompletionWorkerFactory` adapts a real Vite module Worker to that
port and can be passed through `LanguageCompletionService.workerFactory`. Its
Worker entry owns remote provider and provider-module registries. The
deterministic, bounded `language.word` lexical provider is available through the
named `language.word` module and is activated by the browser client handshake,
rather than being registered unconditionally. The service defaults to the
in-process host, so a caller must deliberately opt into the Worker factory.
`createBrowserAlphaEditorSession` makes that selection for product Alpha panes
while direct/test sessions retain the local provider.

After the first full snapshot, `LanguageWorkerWireClient` implements
`LanguageWorkerModelSynchronizer`. The coordinator cancels old-version work
before forwarding each immutable `TextModelChange`; the client sends only
versioned offset/length/text changes, and later requests reference the mirrored
version without carrying document text. A missing version clears the client
mirror state so the next request recovers with a full snapshot.

The Worker server owns a `LanguageWorkerDocumentMirror` backed by Alpha's Piece
Tree. It validates a complete transaction before applying changes in reverse
offset order and captures an immutable snapshot for each request, so later
synchronization cannot mutate an older request's text. A rejected sync clears
the server mirror and returns a terminal sync failure; the coordinator replaces
the poisoned Worker on the next request. The ordered Worker channel is the
delivery guarantee, while full snapshots remain the initialization and recovery
path.

`LanguageCompletionProviderCatalog` is a revisioned immutable metadata snapshot,
separate from executable provider functions. Worker registration and removal
publish the actual registry catalog through a completion-specific side channel.
`LanguageCompletionCatalogWorkerClient` validates and freezes each revision,
provides an explicit first-catalog readiness promise, and invalidates stale or
malformed streams.

A custom completion Worker is prewarmed by `LanguageCompletionService`.
Trigger-character requests wait for its first real catalog, verify the captured
model version, and route only when a remote provider matches. Worker failure or
disposal clears the cached catalog before the next trigger rebuilds both Worker
and metadata.

`LanguageCompletionProviderModuleRegistry` publishes immutable available-module
metadata separately from executable module definitions.
`LanguageCompletionProviderModuleHost` serializes activation per module and
registers every loaded provider batch atomically, so collision or validation
failure cannot expose a partial catalog. Removing or deactivating a module
releases that batch with one registry revision.

The module side-channel carries only module IDs and named Active/Inactive state.
`LanguageCompletionCatalogWorkerClient` activates its required modules before
allowing the first completion request to cross the shared port, then returns
the provider catalog produced by those activations. Malformed catalogs,
activation protocol failure, and required-module failure poison the whole
Worker client; coordinator recovery creates a new Worker and repeats module
activation.

Completion candidates may declare `hasDeferredDetails` while provider-owned
`resolveData` remains inside the provider/Worker realm. The provider worker
caches only the exact successful completion request and resolves by
request/model/provider/item identity. A newer result, model change, provider
replacement, or cancellation rejects stale detail work.

`LanguageCompletionResolveWireClient` and server use a separate cancellable
side-channel over the same ordered port. Resolve output is normalized to
`detail` and `documentation` only, so delayed work cannot rewrite label,
replacement range, or insertion text. The session starts resolution only for
its focused item, aborts it on focus/result/selection changes, and exposes
named Complete/Loading/Failed/Unavailable detail state. The widget projects
resolved text within the focused option without interpreting it as HTML.

`TextSelection` preserves anchor/active direction while exposing an ordered
range. `TextSelectionSet` keeps a stable multi-cursor order and one explicit
primary selection. These are immutable values, not document-global state:
multiple editor instances may present the same `TextModel` with different
selections.

`TextModel.trackRange` owns generic document anchors. Callers must select one
explicit `TrackedRangeStickiness` policy describing whether insertion at each
edge joins the range. Tracked ranges update after text mutation and before the
model change event, follow undo/redo through the same transaction mapping, and
are released individually or with the model.

Every undo step receives a monotonic `transactionId` that remains stable across
its commits, undo, and redo; the model `version` still increments for each
commit. `EditorSelectionController` uses that domain-neutral identity to keep
one editor instance's command selections outside the shared document model.
Commands state their post-transaction anchor/active offsets explicitly.

The controller restores its recorded pre-command selection on undo and its
post-command selection on redo. Other controllers sharing the same model keep
independent tracked selections. If a synchronous listener performs another
model edit before `execute` returns, the command's stated post-selection is
stale; the controller discards it and publishes the safely tracked selection
with `ModelChange` reason.

Commands explicitly choose `CoalesceTyping`, `CoalesceBackspace`, or
`CoalesceDelete` to reuse one nominal `TextEditHistoryGroup`. Consecutive
commands join only when every cursor continues the same adjacent operation.
Typing may begin by replacing selections and may continue with insertions or
forward overwrites. Its merged inverse restores the original selected text.
Single- and multi-cursor inverse edits are merged without copying the document.
If deleting at several cursors would make inverse insertions converge, the
model keeps a separate undo step rather than accepting an ambiguous script.
Explicit selection changes, undo/redo, isolated commands, non-adjacent edits,
operation-mode changes, and external model edits break the group.
`pushUndoStop()` exposes the same boundary directly without creating an empty
history entry.

`EditorSelectionController.beginComposition()` opens one protected history
revision for the current single selection. Every `update` replaces the complete
provisional text and states anchor/active offsets relative to the normalized
composition string. Updates remain observable model versions, but they retain
one transaction identity. `commit` keeps one selection-aware undo step;
`cancel` restores the original text and selection without creating redo
history. A revision may temporarily exceed configured history limits so cancel
remains lossless; commit immediately reapplies those limits. Returning to the
original text leaves no empty undo step.

Ordinary editor commands, selection changes, undo, and redo are rejected while
the session is active. A direct external model edit invalidates the session
before another composition update can mutate text. The browser
`compositionstart`/`compositionupdate`/`compositionend` adapter maps the
browser sequence to that session, while core composition still requires exactly
one selection.

`TextDecorationCollection<TMetadata>` gives one feature owner an independent
set of decorations over a shared model. IDs are monotonic and unique within the
renderer realm. Membership and metadata changes emit `Content`; text
transactions that move anchors emit `Range`. `replaceAll` validates every
range before replacing the prior collection. Core treats metadata as opaque
caller-owned data and never defines CSS classes, colors, diagnostics, tokens,
or rendering policy.

`EditorViewportModel` is the DOM-free layout boundary for a fixed-line-height
view. The browser layer supplies viewport size, measured content width, line
height, and requested scroll coordinates. Alpha owns clamping, content extent,
visible line ranges, and overscanned render ranges. Layout snapshots are
immutable and carry the authoritative model version; every text transaction
therefore invalidates a renderer projection even when the line count does not
change. Resizing or changing line height preserves the fractional top-line
anchor where the new content bounds permit it.

`browser/AlphaEditorViewport` projects that immutable layout into one native
scroll host and an overscanned line layer. It reuses DOM nodes for line indices
that remain inside the render range, updates visible text whenever the model
version advances, and synchronizes clamped common scroll coordinates back to
the DOM. Text is assigned through `textContent`; model content is never parsed
as markup. The component owns its root, internal classes, focus presentation,
model listener, resize observer, and scroll listener, but never owns the shared
`TextModel`.

`AlphaDomTextMeasurer` reads the line layer's computed font, letter spacing,
tab size, and horizontal padding. Canvas measures shaped text segments and
font fallback; tab characters advance to measured space-based stops.
`AlphaLineWidthIndex` takes one bounded synchronous first slice for an Alpha
viewport, then refines remaining non-wrapped line widths in cancellable idle
slices. The measured maximum is an explicit lower bound until that scan
completes; it is never presented as an exact full-model value. An edit during a
pending scan cancels the old generation and restarts against the new model
version. Once complete, each `TextModelChange` maps old affected line groups to
their new model ranges and only those groups are remeasured. A counted width set
keeps the authoritative maximum for the active measurer and clamps horizontal
scrolling when the longest line shrinks. Font changes restart the configured
measurement policy through `refreshFontMetrics`; browser font-loading completion
invokes the same path.

Each rendered row now contains a sticky line-number gutter, text, and a
component-owned overlay. The gutter width is measured from the current
line-count digit width and contributes to horizontal content width.
`AlphaEditorViewport` may observe a caller-owned `EditorSelectionController`.
It marks the primary active line with `.active` and projects all selection
ranges and carets into the overscanned window. Prefix measurement keeps tabs
and shaped glyphs consistent with line-width measurement; selected newlines
receive one measured-space cell so multi-line end-exclusive ranges remain
visible. `.active` is the component-owned active-line visual state, while
`.primary` identifies the primary caret for later presentation; neither relies
on an ARIA-based selector.

`createAlphaDecorationSource<TMetadata>` is the explicit boundary from opaque
common decoration metadata to browser presentation. A caller-provided resolver
selects `SearchMatch`, `ErrorUnderline`, `WarningUnderline`, or omits the
decoration from this renderer. `AlphaEditorViewport` observes multiple sources
without owning them and renders only rectangles intersecting the overscanned
line window. Resolved snapshots are cached per source until its collection
event, so viewport scrolling does not rerun metadata resolvers. Search, error,
and warning presentation consume existing semantic theme tokens in
component-owned CSS.

`LanguageDiagnosticDecorationBridge` observes one typed diagnostic store and
owns a generic `TextDecorationCollection<LanguageDiagnostic>`. Store result,
replacement, clear, and model invalidation become atomic collection
replacement; the bridge never owns the store or model. Because the store's
model listener predates the bridge-owned collection, a text transaction clears
diagnostics before tracked ranges can emit a misleading movement event.
`createAlphaLanguageDiagnosticSource` maps every normalized severity to a
named underline presentation and a diagnostic hover message. Error and Warning use
their severity tokens; Information uses the focus token and Hint uses the
description token with a dotted underline. The viewport projects the highest
severity on each visible logical line as a gutter marker and joins that line's
messages in `AlphaDiagnosticHoverController`'s component-owned rich hover.
Overview-ruler presentation remains a separate browser extension.

Selection and decoration ranges share `createAlphaRangeRectangles`; prefix
measurement, end-exclusive multi-line ranges, selected newline cells, and
render-range clipping therefore cannot drift between the two projections.
The diagnostic adapter does not add arbitrary caller CSS classes. Browser
projection uses `AlphaDecorationLineIndex` to resolve only decorations that can
intersect the rendered logical-line span; it does not rescan the full
collection on every scroll.

`AlphaEditorViewport` additionally owns a document-view minimap. It compresses
the model into at most 160 sampled, non-whitespace density rows and redraws only
after a model revision; it does not create a second model, retain source text,
or rerun semantic analysis. On browsers with WebGL, `AlphaGpuMinimapRenderer`
draws only those density rectangles at device resolution; WebGL absence or context
loss restores the DOM row projection. A primary pointer press maps a proportional
location to the existing viewport scroll position, and dragging continues to map
pointer movement from the owner document until the pointer ends. Existing diagnostic decorations are
condensed into named severity markers through the same snapshot used by the
overview ruler; syntax colors and arbitrary decoration markers remain outside
the minimap contract.

Visible rows also receive component-owned indentation guides. The indentation
projection derives complete visual indentation units from leading spaces/tabs
using the configured editor tab size, then positions the guides with the same
text measurer used by caret and selection geometry. Wrapped continuation rows
do not duplicate guides; no source text or indentation state is retained
outside the visible DOM projection.

`AlphaEditorViewport` announces a single caret/selection precisely, while a
multi-selection update reports the number of cursors or total selected characters
and the primary position through the same atomic live region.

`AlphaEditorViewport.getTargetAtClientPoint` accepts the `clientX/clientY`
shape shared by browser pointer events and returns a `Gutter`, `Text`,
`EmptyContent`, or `AfterLines` target with a valid `TextPosition`. It accounts
for root bounds, sticky gutter, current horizontal/vertical scroll, fixed line
height, line ends, empty lines, and space below the final line. The method is a
query only: it does not focus the editor, capture a pointer, or mutate an
`EditorSelectionController`.

Horizontal hit testing compares the pointer with adjacent caret midpoints using
the active `AlphaTextMeasurer`. Grapheme segmentation prevents emoji and
combining sequences from producing accidental UTF-16-interior carets; the
fallback preserves Unicode code-point boundaries. Tabs therefore use the same
stops as width, selection, and decoration geometry. Measured soft wrapping maps
each grapheme-safe visual fragment through hit testing, selection, decorations,
semantic tokens, scroll virtualization, and vertical navigation. The viewport
now exposes an explicit browser paragraph-direction input (`auto`, `ltr`, or
`rtl`) and applies it to both rendered text and its accessible textarea, so
Chromium owns Unicode shaping instead of `TextModel`. `domTextGeometry.ts`
maps Alpha's UTF-16 source offsets across semantic-token spans to browser
`Range` rectangles and caret positions. In automatic/RTL direction, rendered
selection/caret, decorations, composition anchors, pointer hit testing, and
wrapped vertical cursor movement use that geometry when the browser supplies
layout; a deterministic text-measurer fallback preserves non-layout realms.
Inline decorations that alter advance width and native browser-driven wrapping
remain future view work.

`AlphaPointerSelectionController` consumes hit targets without moving pointer
policy into the Viewport. A primary click replaces the selection with one
caret; Shift-click extends the existing primary anchor. Character drags retain
anchor/active direction. Gutter click/drag selects complete lines and includes
the LF boundary when one exists; upward gutter drags remain backward
selections. Browser double-click detail selects and drags complete word
segments; triple-click detail reuses complete-line drag semantics. Shift keeps
the prior primary anchor while extending to the target word or line boundary.
For a context-menu gesture, a hit inside an existing non-empty selection keeps
that selection; another hit replaces it with one caret. The controller leaves
menu composition and display to its host.

`LanguageBracketColorizationIndex` consumes the same lexical structural-bracket
events as matching, caches nesting state by line, and produces one of six closed
color levels for matched pairs. `AlphaBracketColorizationSource` supplies only
the visible line spans to `AlphaEditorViewport`; the renderer composes them with
semantic-token spans without changing source text. Brackets in strings and
comments never enter the index, unmatched closers remain uncolored, and a model
transaction invalidates the cached nesting before the next projection.

Alt+Shift primary drags create a rectangular column selection through
`createEditorColumnSelectionSet`. The common mapper owns one directional
selection per physical line and makes the active line primary. Columns clamp at
each line end instead of inventing virtual whitespace, so short rows remain in
the operation as collapsed selections. This is browser input policy over the
same multi-selection state and does not use DOM selection or change the model.

`AlphaPointerSelectionControllerOptions.multiCursorModifier` explicitly chooses
`Alt` by default or `ControlOrMeta`. The configured modifier without Shift
retains existing selections and adds the pointer-produced character, word, or
whole-line selection as primary. Clicking an existing range toggles it off
unless it is the last selection; dragging from it replaces it. Identical ranges
deduplicate, and overlapping retained ranges are removed so future edit
commands do not receive overlapping pointer selections. Shift chords remain
ordinary extension gestures instead of acquiring an implicit multi-cursor
meaning.

`getWordSelectionRange` is common-layer policy shared with keyboard navigation
without importing browser code. It selects the complete word-like,
whitespace, or punctuation segment on one line, using `Intl.Segmenter` when
available and a Unicode code-point fallback otherwise. End-of-line positions
select the preceding segment.

`textSegmentation.ts` now owns the shared grapheme boundaries and word segments
used by pointer hit testing, word selection, and cursor navigation.
`navigateEditorCursors` applies character, word, vertical-line, line-boundary,
document-boundary, and page commands to every selection. `Move` and `Extend`
are explicit modes. Vertical results return preferred UTF-16 columns so a
caller can preserve intent across short lines; target columns are clamped to a
grapheme boundary. Exact duplicate results coalesce while retaining primary
mapping.

An active drag owns a collapsed position or complete word `TrackedRange` plus
window-level move/up/cancel/blur listeners and native pointer capture. This
keeps drag state valid across synchronous model transactions. Completion,
cancellation, blur, adapter disposal, or setup failure releases all temporary
ownership. The Viewport and pointer adapter compare the selection controller's
public read-only `textModel` identity and reject cross-document wiring before
it can silently publish a valid-looking position to the wrong model.

`AlphaEditorViewport.getNearestTargetAtClientPoint` is the explicit active-drag
counterpart to strict hit testing. It clamps an outside point to the nearest
viewport edge, while ordinary `getTargetAtClientPoint` continues to reject the
same point. `AlphaPointerAutoScroller` maps horizontal and vertical overflow
independently to a bounded pixels-per-second velocity, advances
`EditorViewportModel` through the Viewport on animation frames, then repeats
nearest hit testing so every character, word, or line selection mode continues
through scrolling. Returning inside, reaching the scroll limit, completing or
cancelling the pointer, blur, and disposal all stop further frames.

During an additive gesture, the adapter owns temporary tracked copies of the
original selection set in addition to the active anchor. It reconstructs their
direction, order, and primary identity after synchronous model transactions,
then applies `combineAlphaPointerSelection`. These resources share the drag
lifecycle and do not change common-layer selection ownership.

`AlphaKeyboardNavigationController` maps local keydown events to those common
commands with explicit Windows/Linux or macOS semantics. Shift extends;
Ctrl/Option word navigation and platform document/line boundary chords are
handled with their platform-specific modifiers. AltGraph, composition events, unknown chords, and events already
handled above the component are ignored. Page distance comes from the current
fixed-line-height viewport. Successful navigation calls
`AlphaEditorViewport.revealPosition`, which uses measured prefix and line
geometry to keep the primary active position visible. Text insertion, deletion,
clipboard, and composition DOM events remain outside this controller.

`AlphaLineCommentController` handles Ctrl/Cmd+`/` only when the active language
configuration declares a line-comment token. The common command toggles the
union of selected physical lines after their indentation, ignores blank rows
when deciding whether a mixed selection should remove comments, and maps every
selection through one isolated undo transaction. Languages without a line rule
leave the browser shortcut untouched. Block comments use their own range-based
command because their selection semantics differ from physical-line toggling.

`AlphaBlockCommentController` handles Shift+Alt+A when the resolved language
configuration declares a block-comment pair. Each non-overlapping selected
range is wrapped or unwrapped in one isolated transaction; collapsed cursors
insert a pair and remain inside it. The common command preserves directional
selections and rejects ambiguous overlapping selections before mutation.

`AlphaLineOperationsController` in `contrib/linesOperations/browser` maps Ctrl/Cmd+Enter and Ctrl/Cmd+Shift+Enter
to insert blank lines after/before selected physical-line groups,
Ctrl/Cmd+Shift+K to delete them, Shift+Alt+ArrowUp/ArrowDown to duplicate
them, and Alt+ArrowUp/ArrowDown to move them. `contrib/linesOperations/browser/linesOperations.ts` computes all
edits against one pre-change snapshot, keeps the document non-empty after
deleting its only line, and routes each operation through an isolated
selection-aware undo step.

`AlphaOccurrenceSelectionController` maps Ctrl/Cmd+D to select a cursor word
or add the next exact occurrence, and Ctrl/Cmd+Shift+L to select every exact
occurrence. The common selector owns Unicode-safe source-word selection,
wraparound, de-duplication, selection ordering, and primary-selection choice;
the browser adapter only routes chords and reveals the new primary cursor.

`AlphaMultiCursorController` maps VS Code-compatible add-cursor chords to
logical physical lines: Ctrl+Alt+Arrow on Windows, Cmd+Alt+Arrow on macOS, and
Ctrl+Shift+Alt+Arrow on Linux. The Linux chord deliberately uses VS Code's
secondary binding so Shift+Alt+Arrow remains available for line duplication.
`common/cursor/cursorInsertion.ts` preserves current selections, clamps columns on shorter
rows, and prevents new carets from duplicating or overlapping them.
It also maps Shift+Alt+I to replace non-empty selected rows with their line-end
carets, including the selected endpoint only when it is not at a following line
start.

`AlphaEditingCommandController` also maps Ctrl/Cmd+L to the same repeated line
selection model as VS Code: it normalizes a selection to the first line start,
extends through one more physical row each time, and includes a non-final line
break by ending at the next line start.

`LanguageBracketMatcher` incrementally scans configured brackets with the same
lexical rules that exclude strings and comments from structural editing. It
matches nested pairs in a bounded line window and invalidates cached suffixes
after text or configuration changes. `AlphaBracketMatchController` observes
collapsed selections and renders the two current ranges as named
`BracketMatch` decorations; non-collapsed selections intentionally clear the
highlight.

`AlphaBracketNavigationController` maps Ctrl/Cmd+Shift+`\\` to go to each
cursor's lexically valid matching bracket. `bracketNavigation.ts` also exposes
the selection transformation for command routing: it selects from the opening
token through the closing token without changing text or recreating lexical
state.

`AlphaBracketEditingController` maps Ctrl/Cmd+Alt+Backspace to remove distinct
matched pairs around collapsed cursors in one isolated transaction. It
canonicalizes post-edit duplicate carets and rejects non-cursor or lexically
invalid bracket candidates without changing the document.

`createTypeTextCommand`, `createBackspaceCommand`, `createDeleteForwardCommand`,
and soft line-boundary deletion commands turn one immutable selection set into a canonical
multi-edit command. Typed text is LF-normalized before result offsets are
calculated. Collapsed deletions remove a complete grapheme or the adjacent line
break; selected ranges are deleted directly. Overlap is rejected before model
mutation, and identical carets produced by adjacent deletions coalesce with
primary identity remapped. Undo still restores the original source selections.

`createSelectionEditCommand` is the shared multi-selection transaction builder
used by ordinary and language-aware edits. `createLanguagePairTypeCommand`
surrounds directional selections, auto-closes configured opening tokens only
at an allowed following character, and emits post-transaction provenance
actions. `LanguageAutoClosingTracker` owns tracked enclosing/closer ranges for
one editor instance. Overtype and paired Backspace require its positive trust,
so a matching closer already present in user-authored text is never treated as
automatically inserted merely because its characters match. The tracker
follows external edits, invalidates entries when their pair changes or all
selections leave it, and never owns the model or selection controller.

`createLanguagePairBackspaceCommand` removes both sides only for a trusted
empty pair while ordinary selections in the same transaction retain normal
grapheme deletion. Multi-character pairs and multi-cursor offset shifts use
the same pre-change command contract and undo ownership as all other edits.
Undo invalidates provenance when the pair disappears; redo restores text and
selection history but deliberately does not invent provenance that is no
longer live.

`createLanguageEnterCommand` evaluates the current resolved `onEnterRules`
first, configured bracket boundaries second, and `indentationRules` last.
Every selection contributes one pre-change replacement to the same command;
Indent, IndentOutdent, Outdent, append text, remove text, directional
selection replacement, and multi-cursor offset shifts therefore share the
canonical transaction path. `BeginCoalescedTyping` breaks the group before
Enter while allowing immediately following typing to join the new undo step.

Tabs/spaces and tab size are editor-instance choices represented by
`EditorIndentationOptions`, not language configuration and not `base`.
Normalization uses visual columns so mixed leading tabs/spaces produce a
canonical result for the selected style. `LanguageLexicalContextSource`
optionally supplies processed line slices; `LanguageLexicalContextIndex` is
the default synchronous source. It lazily reuses
`LanguageLexicalLineScanner`, removes configured bracket tokens only inside
string/comment/ECMAScript regular-expression spans, and invalidates the affected cached suffix after edits.
Configuration identity changes rebuild its scanner.

`LanguageConfiguration.wordPattern` is an optional immutable RegExp contribution.
Its current resolved value drives pointer word selection, Ctrl/Option word
navigation, word deletion, cursor-originated occurrence selection, and
occurrence highlighting. Explicit text selections retain literal occurrence
semantics. Unconfigured languages continue to use shared Unicode segmentation.

`LanguageAutoClosingPair.notIn` currently has the closed `string | comment`
vocabulary supported by the configuration contract. Pair input consults the
same source before auto-closing, while trusted closer overtype remains governed
by `LanguageAutoClosingTracker`. ECMAScript regular-expression literals are
separate structural contexts; embedded-language regions and template
interpolation are not yet distinct lexical contexts.

`AlphaTextInputController` owns a hidden textarea and redirects viewport focus
to it. Non-composition `beforeinput` events route ordinary text, replacement
text, Enter, Backspace, Delete, soft-line deletion, undo, and redo through the common command
boundary; an unmodified Tab inserts `\t`. Successful changes reveal the
primary active position. The stable `.input-focused` root class exposes focus
state to component-owned CSS. Composition and clipboard events are delegated
to separately owned browser controllers; dead-key handling and screen-reader
mirroring remain outside the ordinary input router.

`AlphaTextInputControllerOptions.language` explicitly supplies one language ID
and caller-owned `LanguageConfigurationSource`. The controller resolves the
current revision for every relevant `beforeinput`, so configuration changes do
not require recreating the view. If completion request wiring is also present,
both paths must name the same language. The controller owns neither the
registry nor its registrations. It does own and dispose its optional
`LanguageAutoClosingTracker`, records actions only when the expected model
version actually committed, and passes that tracker back into later pair
commands as the source-of-truth boundary. `insertLineBreak` and
`insertParagraph` resolve the same current language revision and invoke the
common Enter command. Optional top-level `indentation` supplies the
editor-instance style; the default is four spaces. A caller may inject
`language.lexicalContext` for sharing at a document composition root;
otherwise the controller owns one local index and still owns neither model nor
language configuration registry.

`getSelectionTexts` reads ranges in stable selection-set order. Ordinary
`createPasteTextCommand`, `createDistributedPasteTextCommand`, and
`createCutCommand` build isolated selection-aware transactions. Paste and cut
therefore remain separate undo steps instead of joining adjacent typing.

`AlphaClipboardController` writes portable `text/plain`, safe preformatted
`text/html`, and versioned Alpha metadata for multi-selection distribution.
When a current `AlphaSemanticTokenSource` is present, its closed presentation
vocabulary and resolved browser theme colors annotate the HTML copy only;
plain text remains authoritative and unavailable/stale tokens produce escaped
preformatted HTML. Matching Alpha metadata pastes one text per selection;
external text and invalid metadata paste the same text at every selection. When
`text/plain` is absent, the browser adapter extracts deterministic text from
inert HTML without rendering it or accepting script/style content. Clipboard
output uses an explicit platform line-ending policy and model input remains
LF-normalized.

If no text representation is available, Alpha may asynchronously read one
clipboard `File` that has a textual MIME type or known text extension and is at
most 5 MiB. It never opens a file-system path. The result must retain the
captured model version and all selection anchor/active positions before it can
be submitted as an isolated paste command; otherwise it is discarded.

`EditorInput.readOnly` configures one non-mutating Alpha editor instance. The
instance still owns selection, navigation, copy, rendering, and external model
observation, while `EditorSelectionController` rejects execute/undo/redo and
composition entry before any browser command can reach `TextModel`.

`EditorEmptySelectionClipboardPolicy.Line` is the browser default;
`Ignore` explicitly retains native empty-copy behavior.
`getEditorClipboardEntries` resolves each collapsed caret to complete-line text
and a cut range that owns the appropriate following or preceding LF.
`createClipboardCutCommand` merges overlapping/duplicate ranges and maps every
source caret through the resulting deletion. Version 2 Alpha metadata records
line paste mode. `createLinePasteCommand` inserts at target line starts,
preserves original columns, and groups multiple target carets on one line into
one ordered insertion. Mixed modes safely fall back to ordinary selection
paste. User-provided single text files may also be pasted or dropped under the
same bounded asynchronous contract. `AlphaClipboardPasteProvider` accepts only
an immutable event-time textual MIME snapshot; it is frontend-local, evaluates
providers in explicit order, and discards asynchronous output when the captured
model version or selections no longer match. `AlphaUriListPasteProvider` is the
built-in `text/uri-list` adapter and excludes comment lines. When a native
paste event provides neither text nor Alpha metadata, a recognized text file,
or a matching local provider, `AlphaClipboardController` invokes the browser
Async Clipboard rich reader before its plain-text fallback within that paste
gesture. Its result uses the same captured model version and selection gate,
so delayed, denied, empty, or stale reads cannot mutate the document. If a
copy/cut event has no event-owned `clipboardData`, the corresponding Async
writer exports the same `text/plain` and safe `text/html` payload; a cut waits
for a successful write before changing the model. Mutable clipboard commands
remain rejected while an IME composition is active.

`AlphaCompositionController` maps a desktop-style
`compositionstart/update/end` sequence to one `EditorCompositionSession`.
Updates replace the complete provisional string inside one protected revision,
read relative textarea selection when it can be correlated safely, and fall
back to a caret at the normalized text end. End commits one undo step. Escape,
blur, disposal, and shared `IME.disable()` cancel losslessly. Direct model
observation clears a session invalidated by external edits even when its
selection did not move.

The controller exposes composition state through `onDidChange`, projects
stable `.composing` and `.ime-input` classes, and positions the textarea from
`AlphaEditorViewport.getPositionContentCoordinates`. This anchors the native
candidate window to the measured caret across layout changes. Android
replacement deduction, macOS long-press behavior, iOS empty-end anomalies,
clause segmentation, and multi-selection IME remain future platform adapters.
An isolated desktop dead-key commit that is marked composing without a matching
composition session safely follows the ordinary text path. An active empty end commits deletion; Escape/blur are explicit
cancellation signals, while an extra end after closure is ignored.

`EditorCompositionSession.currentRange` exposes the provisional model range
only while its protected revision is active. The Viewport converts it to an
owned temporary `TrackedRange`, so synchronous model events cannot make
projection read stale positions. A separate per-line composition layer reuses
`createAlphaRangeRectangles` for measured tabs, multi-line clipping, and newline
cells. Component-owned `.composing` CSS draws the underline and every commit,
cancel, invalidation, or disposal clears the temporary layer and handle.

## Internal ownership map

```text
TextPosition / TextRange
          ↓
TextModel.applyEdits
          ↓
prepareEdits
  validates and orders one pre-change transaction
          ↓
commitOffsetEdits
  creates inverse edits and applies replacements from the end
          ↓
PieceTreeTextBuffer.replace
  splits/coalesces pieces and updates subtree length/line/piece counts
          ↓
TextModel.scheduleMaintenance
  uses the product scheduler when one is supplied; otherwise compacts synchronously
          ↓
PieceTreeTextBuffer.compactIfNeeded
  reclaims dead source storage without changing model semantics
          ↓
commitOffsetEdits
  increments version, emits TextModelChange
          ↓
TextModelHistory
  owns undo/redo stacks, budgets, grouping, and protected revisions

TextModel.trackRange
          ↓
TrackedRangeCollection
  maps offsets through the complete transaction
          ↓
TextModelChange event
  listeners observe anchors for the committed version

TextModel.createSnapshot
          ↓
LanguageRequestCoordinator.runLatest
  owns request IDs, named latest-wins lanes, and AbortSignal cancellation
          ↓
LanguageWorker.run
  consumes the immutable captured version without owning TextModel
          ↓
current request + current model version gate
  publishes VersionedLanguageResult or discards the late result
          ↓
VersionedLanguageResultStore.accept
  validates the request high-water mark and freezes caller-owned data
          ↓
Result / ModelChanged / Cleared event
  exposes current token or diagnostic state without tracked-range drift
          ↓
LanguageDiagnosticDecorationBridge
  owns a generic collection and clears before tracked-range movement
          ↓
createAlphaLanguageDiagnosticSource
  maps only Error/Warning to component-owned named presentations

EditorSelectionController.execute
          ↓
TextModel transactionId
  stable across grouped edits / undo / redo
          ↓
per-instance selection history
  restores command before / after selections

TextEditHistoryGroup
          ↓
canCoalesceHistoryEdits
  proves cursor count, operation direction, and adjacency
          ↓
coalesceHistoryUndoEdits
  merges inverse inserts or deletes without a document snapshot
          ↓
normalizeInverseEdits
  removes same-offset ambiguity from adjacent deletions

EditorCompositionSession
          ↓
TextModel protected history revision
  keeps the original inverse while provisional text changes
          ↓
commit / cancel
  retains one undo step or restores without redo history

TextDecorationCollection<TMetadata>
          ↓
TextModel.trackRange
  owns position movement only
          ↓
Content / Range event
  projects immutable collection snapshots

EditorViewportModel
          ↓
base/common/layout.ISize
  reuses domain-neutral geometry only
          ↓
visibleLines / renderLines
  owns editor scroll clamping and overscan semantics
          ↓
immutable versioned layout event
  invalidates browser projection on model or geometry changes

AlphaEditorViewport
          ↓
base/browser DOM + geometry
  observes and projects browser state
          ↓
EditorViewportModel layout
  sizes content and reconciles overscanned rows by line index
          ↓
component-owned DOM/CSS
  preserves overlapping row identity and escapes text content

AlphaDomTextMeasurer
          ↓
computed line-layer font + Canvas text shaping
  resolves glyph, letter-spacing, tab-stop, and padding widths
          ↓
AlphaLineWidthIndex
  replaces widths only for affected model line groups
          ↓
EditorViewportModel.setContentWidth
  maintains horizontal extent and clamps native scroll

EditorSelectionController
          ↓
createAlphaSelectionGeometry
  preserves direction, primary identity, and selected line breaks
          ↓
AlphaEditorViewport overlay + gutter state
  projects visible selection rectangles, carets, and active line numbers

TextDecorationCollection<TMetadata>
          ↓
createAlphaDecorationSource resolver
  maps opaque metadata to one named browser presentation
          ↓
createAlphaRangeRectangles
  shares range/newline/clipping geometry with selections
          ↓
AlphaEditorViewport decoration layer
  projects visible rectangles without owning source or collection

clientX / clientY
          ↓
AlphaEditorViewport root bounds + scroll state
  converts client coordinates to fixed-line viewport coordinates
          ↓
hitTestAlphaEditorPoint
  distinguishes gutter/content/after-lines and measures caret midpoints
          ↓
AlphaEditorHitTarget
  returns a valid TextPosition without mutating selection

AlphaPointerSelectionController
          ↓
pointerdown + temporary tracked anchor
  chooses character, word, whole-line, or Shift extension policy
          ↓
getWordSelectionRange
  keeps word-like text, whitespace, punctuation, and Unicode boundaries explicit
          ↓
window pointermove / pointerup / pointercancel / blur
  updates one EditorSelectionController and releases pointer capture

configured Alt or ControlOrMeta gesture
          ↓
temporary tracked base selections
  preserve order, direction, and primary identity through model edits
          ↓
combineAlphaPointerSelection
  toggles, deduplicates, removes overlaps, and appends the active primary

outside client point
          ↓
getAlphaPointerAutoScrollVelocity
  maps each overflow axis to a bounded pixels-per-second velocity
          ↓
AlphaPointerAutoScroller + base/browser/AnimationFrameScheduler
  advances AlphaEditorViewport and repeats nearest-edge hit testing

browser keydown
          ↓
AlphaKeyboardNavigationController platform routing
  chooses one common command and Move/Extend mode
          ↓
navigateEditorCursors + textSegmentation
  moves every selection and retains vertical preferred columns
          ↓
AlphaEditorViewport.revealPosition
  keeps the primary active line and measured horizontal caret visible

textarea beforeinput / plain Tab
          ↓
createTypeTextCommand / createBackspaceCommand / createDeleteForwardCommand
  builds one pre-change multi-edit script and explicit result offsets
          ↓
EditorSelectionController.execute
  commits text and selection-aware history through the canonical model
          ↓
AlphaEditorViewport.revealPosition
  reveals the primary result without transferring model ownership

copy / cut / paste
          ↓
AlphaClipboardController + versioned browser metadata
  keeps DOM clipboard formats outside common
          ↓
getSelectionTexts / isolated paste and cut commands
  preserves stable multi-selection mapping and undo boundaries
          ↓
EditorSelectionController.execute
  commits clipboard edits through the canonical model
          ↓
empty-selection policy + version 2 line metadata
  merges whole-line cuts and preserves target columns on line paste

compositionstart / compositionupdate / compositionend
          ↓
AlphaCompositionController + textarea-relative selection
  maps browser state to normalized complete provisional text
          ↓
EditorCompositionSession
  replaces one protected history revision or cancels losslessly
          ↓
AlphaEditorViewport content coordinates
  anchors the native IME input at the measured active caret
          ↓
EditorCompositionSession.currentRange + tracked Viewport range
  projects a multi-line provisional underline without stale offsets
```

`commitOffsetEdits` is the mutation boundary. Bypassing it would break version,
event, and history consistency. `PieceTreeTextBuffer` is the private storage
boundary; it references immutable original/add buffers through a deterministic
treap whose nodes cache character, line-feed, and piece totals.
`pieceTreeBase.ts` owns node invariants, while `pieceTreeSnapshot.ts` owns
immutable segment capture and offset reads. `OffsetEdit` remains private so
storage offsets cannot leak into the public position-based API.

## Failure and lifecycle semantics

- Invalid positions, reversed ranges, out-of-bounds offsets, and overlapping
  transactions throw before changing text or history.
- Exact replacements are no-ops: they do not increment the version, emit an
  event, or add history.
- A new edit after undo clears redo history.
- History grouping is opt-in. Unsupported or non-adjacent edits create a new
  undo step even when the caller reuses a group.
- History evicts the oldest reachable transactions when either configured
  limit is exceeded. Grouped edits update the stored-text accounting after
  every merge. An edit still commits when its inverse cannot fit.
- An active history revision protects its latest undo entry until commit or
  cancel. Commit reapplies budgets; cancel consumes the entry while restoring
  text and never creates redo history.
- Invalid tracked-range positions fail before registration. Disposed tracked
  ranges reject subsequent reads.
- Invalid post-command selection offsets fail before text mutation.
- `replaceAll` rejects invalid decoration ranges without changing the existing
  collection. Updating an unknown decoration ID throws.
- Disposing an `EditorSelectionController` releases only its selections and
  listener; it never disposes the shared `TextModel`.
- Viewport and pointer-selection construction reject a selection controller
  bound to a different text model.
- Disposing a decoration collection releases only its tracked ranges and model
  listener; it never disposes the shared model or caller metadata.
- Disposing a viewport releases only its model listener; it never disposes the
  shared `TextModel`.
- Disposing `AlphaEditorViewport` disconnects browser observers/listeners and
  removes its root; the shared model, optional selection controller, decoration
  sources, and their collections remain usable.
- Disposing `AlphaPointerSelectionController` releases its active tracked
  anchor, window listeners, and pointer capture without disposing the viewport,
  selection controller, or model.
- Disposing `AlphaTextInputController` removes its hidden textarea and owned
  listeners without disposing the viewport, selection controller, or model.
- Disposing `AlphaClipboardController` removes only its clipboard listeners;
  the textarea input controller normally owns this adapter.
- Disposing `AlphaCompositionController` cancels an active revision, restores
  the prior textarea read-only state, and removes only its state classes and
  listeners.
- Disposing a `LanguageRequestCoordinator` cancels every active lane and
  disposes its worker without owning the observed `TextModel`. An active worker
  failure cancels peer lanes, disposes the failed worker, and lazily creates a
  replacement on the next request.
- A `VersionedLanguageResultStore` rejects stale, duplicate, and superseded
  results and cross-model wiring before state changes. Model edits clear
  accepted results; disposing the store releases only its listener and result,
  never the shared model.
- Disposing a `LanguageDiagnosticDecorationBridge` releases its projected
  collection and store listener, never the diagnostic store or text model.
- Disposal releases listeners and history. All subsequent model access throws
  `ReferenceError`; tracked ranges are disposed while previously created
  snapshots remain readable.

## Current limitations and next layers

The piece tree makes splits, merges, and line-count traversal proportional to
tree depth. Adjacent contiguous source pieces are coalesced, snapshots capture
immutable source segments, history collections are bounded by transaction and
stored-text budgets, and threshold-driven compaction reclaims dead current
storage.

`TextModel` keeps synchronous compaction as its framework-neutral default.
Alpha's browser `BrowserTextModelService` supplies a cancellable idle scheduler,
so its file-backed models never join an edit transaction solely to reclaim
piece-tree storage. One idle callback still compacts the live tree as an
O(document length) operation. Snapshots intentionally retain their captured
source strings until the snapshot itself becomes unreachable; those external
retained sources are not included in `getStatistics`. Truly incremental copying
remains a future optimization for unusually large documents.

The next implementation stages are:

1. incremental piece-tree compaction for unusually large documents; Alpha browser models already defer whole-tree maintenance, while non-wrapped line-width and initial wrapped-line measurement yield after a bounded first slice;
2. composition clause projection work; mobile remains out of scope;
3. TextMate extension-resource discovery; serializable scope-theme selector composition is complete;
4. desktop platform acceptance verification (macOS VoiceOver, then Windows); mobile remains out of scope;
5. parser-grade folding ranges, inline-advance layout evaluation, native browser-driven wrapping, and continuous updates for transformed snippet mirrors while typing;
6. migrate remaining Monaco-only tools through Alpha's public contracts, then
   remove the retired Monaco editor without importing its ownership into Alpha.

Tests under `test/common/` cover normalization, coordinates, atomic edits, failure
atomicity, events, disposal, immutable snapshots, history budgets,
snapshot-safe compaction, 200-operation history replay, adjacent-piece
convergence, selection direction, four tracked-range edge policies, and a
1,000-edit differential comparison against a plain string oracle. Shared-model
tests prove independent editor selections, stable transaction identity,
selection-aware undo/redo, validation before mutation, and reentrant edit
handling. Decoration tests cover stable identity, metadata updates, atomic
replacement, range movement, undo, independent owners, and disposal.
Coalescing tests cover adjacent typing, initial selection replacement, forward
overwrite, Backspace, forward Delete, explicit undo stops, forced group breaks,
converged inverse normalization, history-budget accounting, stable undo
identity, selection restoration, and multi-cursor offset shifts. Composition
tests cover revision updates, commit, cancellation, selection restoration,
zero history budgets, no-op revisions, external invalidation, and ownership
validation. Run `pnpm benchmark:alpha` for the non-gating 2 MiB
construction, scattered-edit, coordinate, snapshot-read, and churn-compaction
baseline.

Language request tests cover immutable captured snapshots, worker reuse,
same-lane supersession, cross-lane concurrency, model-version cancellation,
caller abort, worker crash recovery, both model/coordinator disposal orders,
late-result rejection, and application failure isolation.
Language result tests cover immutable token/diagnostic snapshots, request
high-water ordering, explicit clear, model invalidation, strict span and
metadata validation, failure atomicity, normalization-time edits/reentrancy,
cross-model rejection, coordinator integration, model unavailability, and
independent store disposal.
Diagnostic bridge tests cover initial projection, same-version replacement,
explicit clear, clear-before-range-movement ordering, metadata ownership, and
independent disposal. Browser tests additionally prove Error/Warning named
underlines, explicit Information/Hint omission, model invalidation, and
viewport non-ownership.
Semantic-token tests cover sparse line grouping, same-version replacement,
model invalidation ordering, named presentation resolution, arbitrary worker
type omission, safe exact-text DOM segmentation, invalid snapshot atomicity,
visible-range projection, overlapping row identity, and independent ownership.
Completion tests cover immutable result normalization, composite identities, explicit
range containment, preselection, failure atomicity, same-version focus
retention, cyclic navigation, local cancellation, isolated acceptance/undo,
caret-coordinate projection, ARIA state, keyboard precedence, mouse acceptance,
cross-model rejection, deterministic registry selection, concurrent provider
merge, provider failure isolation, cancellation, Invoke/TriggerCharacter/
IncompleteRefresh dispatch, exact-store wiring, and independent ownership.
Worker-wire tests additionally cross a real structured-clone boundary, rebuild
realm-local coordinate classes, validate snapshot metadata and result ranges,
propagate cancellation, isolate remote errors, retain terminal transport
failure, rebuild a failed worker through the coordinator, and verify bounded
lexical completion. A dedicated Vite build proves that the browser factory
emits a separate module Worker chunk.
Lexical-cache tests prove same-version token/diagnostic sharing, one-line
rescanning in a 1,000-line document, multiline-state convergence, isolated
provider synchronization failure, eager Worker-sync cache updates, and 120
deterministic random edits against a fresh full-scan oracle.
Analysis-delta tests prove request-ID base binding, strict malformed-delta
rejection, missed-response full fallback, full fallback when a splice cannot
reduce transfer, 100 random token/diagnostic round trips, repeated disjoint
multi-splice transactions, two distant bounded edits, and a bounded four-item
transfer for a one-line edit in a document with more than 3,000 tokens.
Token-line tests prove application-gated confirmation, hidden-base
invalidation, one rebuilt plus 999 reused sparse lines, two rebuilt plus 998
reused disjoint lines, a line insertion with one rebuilt plus 1,000 reused
relative payloads, 100 random wire-delta round trips against full token results,
and one resolver call per virtualized line in a 1,000-line document.
Incremental-mirror tests prove cancel-before-sync ordering, full initialization,
delta transfer followed by reference-only requests, Piece Tree multi-edit and
undo replay, immutable old snapshots, version-gap full fallback, atomic invalid
range rejection, and terminal sync failure.
Provider-catalog tests prove immutable revision ordering, metadata validation,
initial readiness, dynamic remote registration, trigger routing with an empty
renderer registry, stale-revision poisoning, catalog clearing on failure, and
Worker/catalog reconstruction on the next trigger.
Provider-module tests additionally prove shared generic lifecycle behavior,
atomic Analysis provider batches, rollback on collisions, required Analysis
activation before the first request, renderer-confirmed result-base forwarding,
and Worker reconstruction after required-module load failure.
Language-configuration tests prove priority/order composition, field clearing,
validation atomicity, registration disposal, built-in ownership, language-ID
cache isolation, JSON/JSONC rule separation, and same-version cache replacement
after a configuration revision.
Pair-editing tests prove following-character policy, single- and multi-token
auto-close, trusted closing-token overtype without a model version, rejection
of user-authored matching closers, directional surrounding, mixed multi-cursor
mapping, paired Backspace, external-edit range movement, selection escape,
independent multi-cursor invalidation, undo/redo provenance semantics, stale
version rejection, disposal, live configuration revisions, and browser input
routing.
Enter tests prove immutable configuration composition and clearing, RegExp
cloning, built-in ownership, explicit rule precedence, documentation-comment
continuation, bracket indent/outdent, increase/decrease/ignore patterns,
mixed-whitespace normalization, selection replacement, multi-cursor mapping,
typing-history boundaries, dynamic revision reads, browser routing, and
validation before mutation.
Lexical-context tests cover structural filtering without deleting surrounding
comment/string text, partial slices, closed and unterminated string boundaries,
line and block comments, multiline state propagation, edit-suffix
invalidation, configuration recompilation, injected ownership validation,
auto-closing `notIn`, and browser input routing.
TextMate adapter tests load real Oniguruma WASM and cover grammar snapshots,
injections, cross-line scopes, incremental state convergence, same-version
grammar revisions, Analysis provider priority, cancellation, and lifecycle
ownership.

Viewport tests cover visible and overscanned line ranges, horizontal and
vertical clamping, resize and line-height anchoring, model-version
invalidation, zero-sized hosts, validation, and no-op suppression.
Browser tests cover safe text projection, initial virtualization, native and
programmatic scrolling, overlapping row identity, visible model updates,
document shrink clamping, geometry updates, measured gutter width, active line
state, multi-line and multi-cursor overlays, tracked selection movement, and
disposal ownership. Pure selection-geometry tests cover backward ranges,
primary caret identity, and newline-only selection endings.
Decoration browser tests cover opaque metadata resolution, named presentation
validation, multi-source ownership, multi-line/newline geometry, render-range
clipping, incremental presentation changes, and tracked movement after model
edits.
Pointer tests cover out-of-viewport rejection, gutter/content classification,
empty and after-lines targets, tab midpoint decisions, grapheme boundaries,
horizontal/vertical scrolling, and client-to-root coordinate conversion.
Pointer-selection tests cover clicks, Shift extension, forward character drag,
whole-line forward/backward gutter drag, pointer ID isolation, capture release,
cancellation, blur, disposal, cross-model rejection, and tracked anchor movement
during external model edits. Word-boundary and click-count tests cover words,
whitespace, punctuation, emoji, combining text, end-of-line behavior,
double-click word drag, triple-click line drag, Shift extension, backward
direction, and tracked word anchors across external edits.
Autoscroll tests cover overflow velocity and caps, nearest-edge targeting,
vertical and horizontal progression, document/line limits, pointer re-entry,
completion, and cancellation.
Pointer multi-cursor tests cover exact Alt and Control-or-Meta matching,
addition, toggle removal, last-selection retention, primary changes,
deduplication, overlap replacement, word selection, Shift precedence, tracked
base selections across external edits, and invalid configuration.
Cursor-navigation tests cover grapheme-safe character movement, range collapse,
Shift extension, cross-line word movement, preferred columns, line/page/document
boundaries, multi-selection primary mapping, duplicate convergence, platform
chords, composition/AltGraph rejection, external reset, reveal, ownership, and
invalid requests.
Edit-command and text-input tests cover normalized typing, multiple selections,
grapheme and cross-line deletion, converged carets, history restoration,
textarea focus, beforeinput routing, Tab, undo/redo, unsupported composition
events, cross-model rejection, reveal, and disposal.
Clipboard tests cover plain text, Alpha multi-selection metadata, distributed
and repeated paste, invalid metadata fallback, cut, isolated undo,
empty-selection policy, complete-line source ranges, final/only-line cut,
duplicate and overlapping cut merge, same-line target grouping, mixed-mode
fallback, line-ending normalization, reveal, and disposal.
Browser composition tests cover provisional replacement, relative selection,
one-step commit/undo, candidate coordinates, Escape/blur/disposal cancellation,
shared IME state, multi-cursor rejection, normalized line breaks, state events,
external invalidation without a selection move, active empty-end deletion,
stray end suppression, and tracked single/multi-line underline cleanup.
Width tests cover longest-line shrink/growth, same-line multi-edit grouping,
font refresh, and a 400-transaction differential run with line changes,
undo, and redo against full document scans.
