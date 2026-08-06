# Gama Editor

> 文件级目录映射、Alpha/VS Code 对照和装配边界见 [`editor-architecture.md`](./editor-architecture.md)。本文记录实现契约、当前行为、测试证据和已知限制。

Gama owns the structured-document domain. Its `common` layer is DOM-free and does not depend on Workbench, Electron, Alpha, or an external editor runtime. `GamaEditorPane` is the Workbench pane, `GamaEditorSession` owns one structured-document browser session, and `TextEditorWidget` is only the embedded Alpha-backed editor for one `textBlock`.

The current core provides:

- schema-validated immutable document trees;
- schema node groups plus minimum/maximum child cardinality, with one shared
  `canContainChild`/`isLeafNode` contract for custom academic/full nodes;
- ordered schema content terms with optional/repeated type or group slots, so
  academic nodes can enforce title/abstract/section structure before rendering;
- a domain-neutral heading outline query with configurable heading selection,
  levels, and labels, exposed from the browser pane;
- stable node identities for block and inline structure;
- atomic transactions for text replacement, node insertion/deletion/movement,
  attribute updates, and text marks;
- block commands for split, join, insertion, movement, and inline-run mark
  toggling;
- semantic list projection (`ul`/`ol`/`li`) with list-item splitting,
  joining, indentation, and outdentation;
- blockquote wrapping/unwrapping and horizontal-rule insertion through common
  structural commands and the browser toolbar;
- schema-validated block-type transactions and a GamaEditorSession-owned block toolbar for
  paragraph, heading, bullet-list, and ordered-list formats;
- schema-backed table/row/cell nodes, rectangular table insertion, cell-order
  navigation, append-on-last-cell Tab, and undoable row/column insertion and
  deletion;
- selection-aware `link` marks with validated `href` attributes, toolbar and
  Mod-K set/update actions, unlink removal, and anchor projection in the rich
  surface;
- ProseMirror-style stored marks for collapsed selections: a cursor-level
  bold/italic/link toggle is kept as insertion state and applied to subsequent
  typing, paste, and IME replacement;
- selection-aware inline image insertion/rendering; image files pasted into a
  GamaEditorSession text surface are read as data URLs, replace the active text selection,
  and commit as `image` nodes;
- profile-owned inline atomic nodes through `inlineNodeViews` and generic
  insertion commands; the `contrib/citation` capability supplies `citation`
  atoms, bibliography/reference nodes, reference resolution, and its toolbar
  actions without adding citation semantics to GamaEditorSession common;
- inline `NodeSelection` for images: clicking an image projects a selected DOM
  state, Backspace/Delete removes it through a common command, and undo restores
  both the image and its node selection;
- adjacent `image` and `hardBreak` nodes can be removed through the same common
  inline-boundary transaction path used by Backspace/Delete;
- Shift+Enter and `insertLineBreak` create `hardBreak` nodes, while a
  non-collapsed selection can remove text and inline nodes in one transaction;
- range replacement and multiline paste commands that preserve inline runs and
  create sibling blocks;
- cross-block text selections across contiguous sibling paragraph/heading
  blocks, including deletion, replacement, multiline paste, and undoable
  removal of intermediate blocks;
- common plain-text extraction for inline and cross-block selections, with
  rich-surface copy/cut routed through GamaEditorSession's model transactions;
- whole-document `AllSelection` with structured/plain clipboard extraction,
  document replacement, and deletion back to an editable empty paragraph;
- GamaEditorSession-owned structured clipboard fragments with a versioned custom MIME
  envelope, schema validation, node-id remapping on insertion, and plain-text
  clipboard fallback for other applications;
- transaction-level undo/redo with selection snapshots, implicit selection
  mapping through text/node steps, and explicit history grouping for typing and
  deletion; the browser pane maps Mod-Z, Mod-Y, and Mod-Shift-Z to that history
  and restores the corresponding editor focus/range;
- versioned JSON transaction envelopes for all document steps, selections,
  stored marks, and JSON-safe metadata, plus a remote-apply model boundary;
- one-sided pending-local rebase in `contrib/collaboration`: stable node targets
  survive remote changes, text offsets and sibling indices are shifted, local
  selections are remapped, and steps whose targets were remotely deleted are
  reported as dropped;
- `DocumentCollaborationSession` keeps canonical and optimistic snapshots
  separate, accumulates pending local steps into a replayable update, validates
  document versions, and exposes remote/acknowledgement changes through a
  disposable event boundary;
- versioned collaboration envelopes in `envelopeSerialization.ts` wrap the
  existing transaction envelope and validate client, sequence, base-version,
  and server-version fields before a transport adapter accepts them;
- immutable transaction metadata via `withMeta`/`getMeta`; builder methods and
  history grouping preserve semantic input/command tags, while inverse history
  does not inherit metadata that describes the original user action;
- `DocumentPluginKey`/`DocumentPlugin` state extensions that update atomically
  for user edits, undo, redo, reset, and (when implemented by a plugin) direct
  selection changes; plugin failures happen before model commit, so they cannot
  leave the document, version, selection, or history partially advanced;
- optional `filterTransaction` hooks let domain plugins reject user, undo, or
  redo transactions before document/history mutation, keeping read-only and
  collaboration policies out of the browser pane;
- DOM-free `DocumentDecoration` and immutable `DocumentDecorationSet` ranges;
  the transaction mapping is computed once and reused for search hits,
  diagnostics, references, or remote cursors, while ranges whose text no longer
  exists are dropped; plugins can expose their own sets without merging source
  identities, and the browser view projects valid ranges onto editable runs;
- ProseMirror-style schema-aware node sizes and absolute document positions;
  `documentPointToPosition`, `documentPositionToPoint`, and
  `resolveDocumentPosition` preserve a deterministic bias at adjacent text and
  structural boundaries, including nested blocks and table cells;
- versioned JSON serialization and strict deserialization.

The Gama document model is deliberately separate from Alpha's line-oriented
`TextModel`. `BrowserDocumentModelService` resolves one `DocumentModelReference`; `GamaEditorPane` hosts the corresponding `GamaEditorSession`; `DocumentWorkingCopy` adapts Gama serialization, dirty/revert/conflict state, and untitled Save As to the shared Workbench working-copy contract.
Alpha's corresponding editor surface is a `codeBlock`; Gama deliberately names the document node `textBlock`. It is a Gama-owned block whose content is zero or one plain `text` child. It is a text block in Gama's document model, not an embedded Alpha document; its language is a block attribute. The browser widget may project that text through the shared `IEmbeddedTextEditor` boundary, and `AlphaEmbeddedTextEditorFactory` supplies the implementation backed by Alpha's `CodeEditorWidget`. Gama owns the block identity and transactions; Alpha never depends on Gama document types. Gama common remains independent of Alpha and can fall back to its own text surface when no factory is supplied.
`DocumentSchema` validates custom top-node definitions as well as the default
`doc` schema; transaction application never assumes that the root is named
`doc`.
Node `content` terms provide an ordered sequence of typed or grouped child
slots with explicit `min`/`max` repetition. Incomplete assembly relaxes slot
minimums only until the atomic transaction reaches final document validation;
child order and declared types remain schema-owned.
`createNode` and transaction insertion may create an intentionally incomplete
composite fragment while one atomic transaction assembles it; strict
`DocumentSchema.validate` is still required before a document snapshot can be
committed. `validateFragment` remains strict by default and exposes an explicit
`allowIncompleteContent` option only for this assembly boundary.
`GamaEditorSessionOptions.schema` lets a product provide the corresponding
custom schema, while `nodeViews` supplies browser-owned projections for domain
nodes without adding those node types or DOM dependencies to `common`. A node
view may return a reusable `{ element, update, dispose }` handle; the Pane owns
that lifecycle and exposes a `renderChildren` callback for composing default
GamaEditorSession child projections.
`GamaEditorSessionOptions.inlineNodeViews` is the atomic-inline counterpart:
the common model owns the node and selection semantics, while a profile owns
its label, accessibility, and click projection. `toolbarActions` provides the
matching browser command extension point.
`GamaEditorSessionOptions.createEmptyDocument` is the matching lifecycle
boundary for a profile's canonical new-document shape; the same factory is
used by `DocumentWorkingCopy` during empty-resource revert/reload, while
non-empty plain text still follows the generic paragraph migration path.
`GamaEditorProfile` groups the profile matcher, schema factory, empty-document
factory, node views, toolbar actions, and plugins. `createGamaEditorPaneOptions`
materializes that group while the Workbench composition root injects text-file,
working-copy, and embedded line-editor services. The Academic Gama contribution selects
from a profile list, so adding another structured document kind does not require
moving profile-specific schema or matching logic into the common pane.
Each profile also owns a stable `editorId`; profiles with different schemas must
not share one Workbench pane instance, because the pane's schema is fixed for
its lifetime. The Workbench passes the selected input into pane construction so
the contribution can enforce that profile boundary before creating the pane.
`buildDocumentOutline` follows heading levels rather than DOM nesting, so
wrapper nodes such as Academic `title` and `section` remain presentation
concerns while outline parentage stays a stable common-layer result.
`DocumentOutlineNavigator` is an optional browser component that renders that
result as a navigable list and delegates focus/reveal back to the Pane;
Academic enables it through its contribution, while the common layer owns no
DOM or navigation policy.
The Academic profile composes the default `doc` schema through
`createAcademicDocumentSchema`, adding ordered `title`, `abstract`, and
`section` wrappers while retaining ordinary block children for migration and
plain-document compatibility. `academic/browser/nodeViews.ts` owns the
profile wrapper projections; `contrib/citation` owns citation/reference
schema, commands, inline projection, and reference-index state. Headings,
paragraphs, marks, selection, and input remain GamaEditorSession-owned child projections.
`DocumentSchema.getNodeSpecs` and `getMarkSpecs` are the
explicit snapshot boundary for domain-owned schema composition.

The browser pane projects unmarked single-run paragraphs to a lightweight
textarea. Paragraphs containing multiple inline runs, marks, hard breaks, or
images use GamaEditorSession's run-based `contenteditable` surface; each rendered run keeps
its document node identity so input and selection can be mapped back to the
common model. `beforeinput` handles text insertion, deletion, paste, and
paragraph splitting through common commands. Textarea and rich-surface IME
composition stays provisional in the DOM and commits through one
metadata-bearing `composition` history transaction; cancellation restores the
last GamaEditorSession snapshot. Shift+Enter is projected
to a common `hardBreak` transaction, and selection deletion is likewise routed
through common inline replacement. Table-cell Tab navigation is a pane-level
focus projection over the common `findAdjacentTableCell` query;
structural row/column changes remain common transactions, and the toolbar
exposes those same transactions for the active cell. Link actions preserve the
active DOM selection when the toolbar is clicked, while the common mark command
owns range splitting, attribute replacement, removal, and history mapping.
Rich-text selection mapping also spans sibling block surfaces, so deletion and
multiline paste can use one common transaction across those blocks.
GamaEditorSession clipboard fragments are extracted from the common selection model and
serialized by `serializeDocumentFragment`; the browser layer only transports
the custom MIME payload and falls back to `text/plain` when the payload is
missing or unsupported. Paste validation and fresh identity allocation stay in
the common layer, so browser HTML is never treated as trusted document data.
`DocumentTransaction.withSelection(undefined)` is an explicit selection-clear
operation, distinct from a transaction that leaves selection mapping to the
model; this keeps whole-document deletion and other structural resets from
resurrecting stale selections.
`serializeDocumentTransaction`/`deserializeDocumentTransaction` are the
transport-neutral protocol for replay and collaboration adapters.
`contrib/collaboration/common/rebase.ts` provides the one-sided
`rebaseDocumentTransaction` primitive for a pending local transaction and a
remote transaction that share the same base snapshot. It preserves stable
structural targets, shifts text and sibling coordinates, keeps local
transaction dependencies, and exposes dropped steps when a remote deletion
removes their target. The primitive deliberately does not choose server order,
provide client-id tie breaking, or implement a complete OT/CRDT session;
`DocumentCollaborationSession` provides the common-layer boundary: it owns the
canonical version, an optimistic document, a cumulative pending transaction,
and explicit remote/acknowledgement envelopes. `envelopeSerialization.ts`
wraps those fields around the existing transaction serialization protocol. A
browser or transport adapter can consume its `onDidChange` event and send the
validated envelope. It still does not choose server order or implement
client-id conflict arbitration, and `DocumentModel.dispatchRemote` remains a
low-level API that expects an already-transformed transaction.

GamaEditorSession plugins are common-layer state extensions, not browser contributions.
`DocumentPluginState.apply` receives immutable before/after document snapshots,
the metadata-bearing transaction, selection snapshots, origin, and model versions. A plugin may
also implement `applySelection` when its derived state depends on selection
changes that do not modify the document. `GamaEditorSessionOptions.plugins`
passes these extensions into every model created by the pane; academic/full
features can therefore register outline, reference, search, or collaboration
state without placing that ownership in the pane or in the base layer.
Plugins may additionally provide a `decorations` projection. `DocumentModel`
keeps each plugin-owned `DocumentDecorationSet` separate, while
`GamaEditorSession` resolves its identity-based ranges to text-node spans,
projects class names and safe `data-*` attributes, and upgrades a plain
single-run paragraph to the rich surface only when a decoration needs it.
`DocumentTransactionMapping` is the common mapping boundary used by decoration
sets and selection mapping; views do not recompute ranges from DOM offsets.
Absolute positions are a common coordinate protocol only; identity-based
`DocumentPoint` remains the durable selection/decoration anchor so replacing a
node can still map or drop ranges without relying on DOM lifetime.
