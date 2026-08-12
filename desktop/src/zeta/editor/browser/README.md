# Aster Editor browser layer

This directory owns Aster's native browser projection of contracts from
`../common/`. It may import `base/common`, `base/browser`, and editor common contracts.
Neither `editor/common` nor `base` may import this layer.

Language presentation is an optional consumer of this layer and lives in
`./view`, `./input`, and feature-owned `../contrib/*/browser` directories. `EditorPart` composes it through
`ILanguageFeaturesService`; the code editor widget and text model do not import
language providers, grammars, diagnostics, or completion services.

Browser-owned responsibilities include:

- viewport observation and DOM virtualization;
- textarea, keyboard, clipboard, drag/drop, and composition-event adapters;
- font and glyph measurement;
- DOM selection, focus, scrolling, and pointer projection;
- ARIA and screen-reader surfaces;
- presentation of editor-owned decorations and semantic tokens.

When its hidden textarea is focused, `TextInputController` mirrors the
current document and primary selection into that native control. This gives
screen readers a standard multiline text surface while the authoritative text,
transactions, and multi-selection state remain in `editor/common`. The mirror
is released on blur and deliberately pauses for IME composition, whose
short-lived textarea value is required for candidate-selection semantics.
Native selection changes flow back into one primary Aster selection only when
they differ from the already mirrored primary selection, so synchronization
does not collapse ordinary editor multi-cursor state.
When multiple selections exist, the textarea retains the primary native range
and exposes the full count plus primary position through `aria-description`.
Forced-colors CSS maps focus, selection, caret, and diagnostic marks to system
colors instead of relying on theme contrast.

Pure text, history, selection, decoration identity, composition transaction
semantics, and DOM-independent layout math remain in `editor/common`.
Workbench parts may host the browser editor and own its external box, but must
not reach through its component internals. Visual rules follow
[`docs/ui-styling-ownership.md`](../../../../../docs/ui-styling-ownership.md).

Aster is a sibling structured-editor domain. Aster browser code must not import Aster or expose its document types through the line-editor contracts.

## Current implementation

`CodeEditorWidget` is the canonical browser editing surface consumed by editor parts and
embedded widgets. It owns one `EditorViewport`, one `TextInputController`, keyboard and
pointer navigation. Its caller retains the shared `TextModel` and the editor-local
`EditorSelectionController`, so disposing a widget never disposes document state and multiple
editors may safely project the same model. `EditorPart` adds language, folding, diagnostic,
save, command, and optional text-drop controllers around this surface; product widgets must
consume the `CodeEditorWidget` rather than assembling viewport and native-input internals
themselves. `CodeEditorWidget` also owns optional placeholder composition, while viewport
padding is canonical editor geometry shared by row projection, hit testing, scrolling, and
placeholder placement.

`EditorViewport` is the first native browser projection. It consumes
`EditorViewportModel`, creates one component-owned scroll surface, and renders
only the overscanned line range. Overlapping lines retain their DOM identity
while scrolling. A new model version updates visible row text synchronously,
and a shrinking document clamps both the common viewport state and native DOM
scroll coordinates.

`DiffEditorWidget` is a separate read-only review projection rather than two
independently scrolling editor instances. Its caller owns a `DiffModel`, which
observes caller-owned original and modified `TextModel` values and accepts only
results pinned to their current versions. Electron/Web App Server hosts use
`RustDiffComputationService`, which adapts the bounded Rust Myers projection,
hunk ranges, and grapheme-safe inline ranges to Aster's UTF-16 model. A host
without the Rust diff transport fails explicitly when it tries to construct
the Aster diff pane. The widget owns one virtualized scroll surface and
side-by-side DOM rows, but owns neither text nor computation. Editing,
selection history,
syntax processing, and model persistence stay with ordinary Aster editor
parts. `nextChange`/`previousChange` and F7/Shift+F7 wrap over changed rows,
add the component-owned `.active` state, and announce the original/modified
location through the diff view's live region.
`DiffEditorInput` gives the Workbench one synthetic tab resource while
retaining two ordinary caller-owned text-resource inputs. `DiffEditorPane`
acquires and releases both `TextModelReference` values, then hosts this
view; it neither turns the synthetic URI into a text model nor owns either
model's save/revert lifecycle.

Document views also own an optional `EditorMinimap` overlay. Its bounded
`createMinimapRows` projection samples no more than 160 density rows and
retains no document text. When WebGL is available, `GpuMinimapRenderer`
draws that bounded density projection at device resolution; its context-loss and
unsupported-browser path retains the existing DOM row projection. The minimap is
a navigation preview rather than a second text renderer: a primary pointer press and
subsequent document-level drag map directly to canonical
viewport scroll state. Embedded views default it off, and it has no model,
selection, semantic-token, or Rust-service ownership.

`EditorViewport` also projects diagnostic severity markers into the
document minimap by reusing the existing decoration snapshot and overview-ruler
aggregation. They carry only a closed named severity and proportional line
span; minimap projection never retains diagnostic text, source text, or an
syntax result. Arbitrary caller decoration markers and syntax-color minimaps
remain outside this browser contract.

`EditorViewport` also projects indentation guides only for visible first
fragments of logical lines. `contrib/indentation/browser/indentation.ts`
identifies complete
visual indentation units from leading tabs/spaces, while `TextMeasurer`
places each guide at the same coordinate system as carets and selections.
Continuation fragments never duplicate a guide, and scrolling does not retain
offscreen source text.

Nested bracket colors follow a separate `LanguageBracketColorizationIndex`
behind `BracketColorizationSource`. It supplies only lexical structural
bracket ranges for the requested visible line, so strings/comments and
unmatched closers remain uncolored. The token renderer composes bracket level
classes with existing semantic-token spans rather than making a second text DOM
or mutating source content.

With `EditorLineWrapping.On`, `VisualLineProjection` converts each
logical line into measured grapheme-safe visual rows. The viewport virtualizes
those rows directly: continuation rows keep a blank gutter, horizontal scroll
is disabled, and text/semantic-token fragments, selection and decoration
rectangles, composition anchors, pointer hit testing, reveal, and vertical
keyboard navigation all use the same version-bound projection. `TextModel`
remains the only document authority; projection rebuilds never create a shadow
model or alter history.

Code folding is an editor contribution under `../contrib/folding/browser`,
following VS Code's `editor/contrib/folding/browser` boundary. The common
visual projection accepts a feature-neutral `EditorLineVisibilitySource`; the
browser `VisibleLineProjection` composes that source with soft wrapping.
The folding contribution owns range data, fold state, hidden-line derivation,
commands, and folding presentation styles. `EditorFoldingModel` owns tracked
physical-line fold regions. Aster supplies
both indentation-derived ranges and synchronous lexical ranges for `{}`, `[]`,
cross-line block comments, and configured named regions; their deterministic
merge keeps only nested or disjoint spans. User-created manual ranges persist
through provider refreshes. Language providers can later replace only their
ranges without taking ownership of fold state. `EditorHiddenRangeModel` derives
hidden physical lines from collapsed regions; the generic visible-line
projection filters wrapped rows and retains a header-row anchor for temporarily
hidden selections.
`FoldingController` maps Ctrl+Shift+`[` / `]` on Windows and Linux and
Cmd+Option+`[` / `]` on macOS, while gutter controls use the same model. A
collapse relocates active cursors hidden by that range to its header, before
the viewport is reprojected. Ctrl/Cmd+K then Ctrl/Cmd+0 collapses all current
regions; Ctrl/Cmd+K then Ctrl/Cmd+J expands them all; `[` / `]` recursively
collapse/expand the innermost containing hierarchy; `1`–`9` select a fold
level; `,` / `.` create/remove a manual range.

`TextDropController` accepts external `text/plain` drags and safely reduces
`text/html` to inert text when plain text is unavailable. It inserts at the
viewport's grapheme-safe hit target as one ordinary editor transaction, using
LF-normalized paste semantics. One bounded textual file is supported separately;
non-text drops remain available to the hosting workbench.

`EditorInput.readOnly` flows into `EditorSelectionController` rather than
being treated as a textarea-only styling flag. The controller rejects every
document command, undo/redo, and composition entry, so all browser adapters
that submit through it retain selection/navigation/copy behavior without a
path that can mutate the shared model.

Each virtual row owns a sticky line-number gutter, a text node, and a
presentation overlay. Gutter width is derived from the current line-count digit
width and participates in authoritative horizontal content measurement.
`EditorViewport` may observe one `EditorSelectionController`; it projects
the controller's primary active line through `.active`, multi-line selected
ranges as overlay rectangles, and every active edge as a caret. The controller
remains caller-owned.

`createAsterSelectionGeometry` is the DOM-independent browser geometry seam.
It preserves anchor/active direction and primary-selection identity while
clipping output to the current render range. Horizontal positions are measured
from line prefixes using the active `TextMeasurer`. A selected line break
occupies one measured space cell so an end-exclusive multi-line range remains
visible even when the following endpoint is at column zero.

`createAsterDecorationSource` adapts one caller-owned
`TextDecorationCollection<TMetadata>` without interpreting metadata in common
code. Its resolver explicitly maps each snapshot to `search-match`,
`error-underline`, `warning-underline`, or no browser presentation. The
viewport observes any number of these sources and projects only their visible
rectangles into a decoration layer below text and selection. It owns the event
registrations, not the sources or collections. Resolved source snapshots are
cached until the corresponding collection event, so scrolling does not rerun
caller metadata resolvers.

`LanguageDiagnosticDecorationBridge` converts one current-version diagnostic
store into a caller-owned generic decoration collection.
`createAsterLanguageDiagnosticSource` then maps every normalized severity to a
component-owned underline. Error and Warning use their severity tokens;
Information uses the focus token and Hint uses a dotted description-token
underline. The projection retains an optional native tooltip with diagnostic
source, code, and message. On the first visible row of each logical line, the
viewport also renders the line's highest-severity diagnostic as a gutter marker.
`DiagnosticHoverController` turns its current joined messages into a
component-owned, non-modal rich hover and closes it on scroll. The viewport
owns neither bridge, collection, store, nor model.

`DiagnosticNavigationController` uses F8 and Shift+F8 to select and reveal
the next or previous current-version diagnostic, wrapping at either end. Every
move also announces the selected severity, source/code when present, and message
through the viewport live region.

`createAsterSemanticTokenSource` adapts one caller-owned
`LanguageTokenLineIndex` to Aster's closed browser presentation vocabulary.
Unknown worker token types are omitted by default; custom language resolvers
may inspect token modifiers but must still return a named Aster presentation,
never an arbitrary CSS class. `getLineTokens` resolves one indexed line on
demand. The viewport resolves only its target virtual range before replacing
any row, so offscreen lines do not invoke the resolver and a resolver failure
cannot partially update the visible window.

`projectAsterSemanticTokenLine` validates each line snapshot before mutation,
then creates text nodes and component-owned spans without parsing HTML. Joined
DOM text must equal the model line exactly. Same-version token replacement
rerenders visible line content without rebuilding rows; model invalidation
removes stale spans before the normal model-version projection. The viewport
owns only its event registration, not the source, common index, result store,
or model. Semantic foregrounds come from registered
`editor.token.*Foreground` theme colors owned by editor presentation. Legacy
`editor.semanticToken.*Foreground` overrides are normalized only at the user-theme boundary.

`CompletionWidget` projects one caller-owned
`LanguageCompletionSessionController` below the viewport's measured trigger
position. It creates only component-owned listbox/option DOM, maps completion
kinds through a closed text vocabulary, and projects `.visible` and `.focused`
alongside ARIA state. The hidden textarea intercepts ArrowUp/ArrowDown,
Enter/Tab, and Escape only while a session is active. Mouse acceptance happens
on mousedown so the synchronous focus rerender cannot detach the target before
the edit is accepted.

`TextInputControllerOptions.completion` opts into this widget through its
caller-owned `session`. Its optional `requests` contract adds one
`LanguageCompletionService`, language ID, and error observer. Ctrl+Space sends
an explicit Invoke request. Registered trigger characters send after the input
transaction using the new version/caret; ordinary input refreshes an active
incomplete result. Requests require one collapsed selection, and request errors
do not undo a successful text edit. The session must observe the exact
service-owned result store.

The input controller owns the browser widget but not the common session,
service, registry, result store, selection controller, or model. Session
disposal hides the widget; widget disposal restores prior textarea ARIA
attributes. Visual geometry and interaction state live in
`../contrib/suggest/browser/media/completionWidget.css` and consume existing dialog, list-selection,
foreground, border, and shadow theme tokens.

`FindController` owns the per-session find/replace dialog and keyboard
routing over `common/textModelSearch.ts` and `common/textSearchCommands.ts`.
It projects matches through a caller-owned `TextDecorationCollection`; the
viewport consumes that collection only through `SearchMatch`. Closing or
disposing the controller clears this projection without changing the query or
document. A non-empty selection at opening becomes a `NeverGrowsAtEdges`
tracked scope. The Find in Selection control and Alt+L reuse that scope for
both navigation and replace-all after match navigation has changed the editor
selection. `media/findWidget.css` owns the dialog geometry and its `.visible`
and `.checked` interaction states, while ARIA attributes expose the equivalent
accessible state.

`GotoLineController` owns Aster's Ctrl+G (Command+G on macOS) line/column dialog. Its common
parser supports one-based `line[:column]`, backward negative values, and `::`
UTF-16 offsets; while the dialog is open it previews by revealing the parsed
position without mutating selections. Enter commits one collapsed primary
selection, while Escape restores the pre-dialog scroll position. Its geometry
and interaction states are owned by `media/gotoLineWidget.css`.

`LineCommentController` consumes Ctrl/Cmd+`/` only when the resolved
language configuration has a line-comment token. It delegates all mutation,
selection mapping, and undo grouping to `common/lineCommentCommands.ts`; the
browser adapter only handles the key event and reveal. Unsupported languages
keep the shortcut available to other browser or workbench handlers.

`BlockCommentController` maps Shift+Alt+A to the configured block-comment
pair. Its common command handles range wrapping, pair removal, collapsed
cursor placement, selection mapping, and undo; the browser adapter only
filters the platform chord and reveals the resulting primary cursor.

`LineOperationsController` under `../contrib/linesOperations/browser` handles Ctrl/Cmd+Enter,
Ctrl/Cmd+Shift+Enter, Ctrl/Cmd+Shift+K, Shift+Alt+ArrowUp/ArrowDown, and
Alt+ArrowUp/ArrowDown without browser-native text mutation. Its line-operation
commands insert, delete, duplicate, or move the union of selected physical
lines in one selection-aware transaction, then the adapter reveals the
resulting primary cursor.

`LineJoinController` maps Ctrl+J (Command+J on macOS) to `common/lineJoin.ts`. The common
command reduces overlapping cursor/range targets before replacing whole
physical-line spans, removes following indentation, retains a single separator
when neighboring fragments contain text, and keeps the resulting selections in
one isolated undo transaction. The browser adapter owns only chord filtering
and reveal.

`TransposeController` maps VS Code's macOS Ctrl+T chord to
`common/cursor/cursorTranspose.ts`. It swaps complete graphemes rather than UTF-16 code
units, supports a line break at a physical-line start, excludes range
selections, and resolves overlapping multi-cursor edits before the one
isolated transaction.

`WordWrapController` maps Alt+Z to a viewport-local word-wrap toggle.
`EditorViewport` always virtualizes through its visual-line source, so
switching wrapping rebuilds row geometry, scroll limits, hit testing, selection
projection, and rendering without changing the model or re-creating the editor.
The `.word-wrapped` root class reflects that component-owned presentation state.

`OccurrenceSelectionController` handles Ctrl/Cmd+D and Ctrl/Cmd+Shift+L
without browser-native selection logic. The common selector resolves one
Unicode-safe source word for a collapsed primary cursor, then adds the next
exact match or replaces the set with every exact match; the adapter only
updates and reveals the live selections.

`OccurrenceHighlightController` keeps a separate caller-owned decoration
collection for the primary cursor word or an explicit single-line selection.
Cursor words use Unicode whole-word matching; explicit selections use literal
exact matching. This presentation never changes the selection set and clears
for whitespace, punctuation, or multiline selections.

`MultiCursorController` adds one logical-line caret above or below every
existing active cursor. Chords follow Windows (Ctrl+Alt), macOS (Cmd+Alt), and
Linux's non-conflicting VS Code secondary binding (Ctrl+Shift+Alt); the common
command clamps columns and rejects duplicate or overlapping new carets.
Shift+Alt+I replaces non-empty selected rows with their VS Code-compatible
line-end carets through the same DOM-free common command. Those actions and
occurrence-selection changes record a bounded cursor-only history;
`CursorUndoController` restores it with Ctrl+U on Windows/Linux or Cmd+U
on macOS without changing the document's undo stack.

`BracketMatchController` is a presentation adapter over the common
`LanguageBracketMatcher`. It projects only the current collapsed-cursor pair
as two `BracketMatch` decorations, reuses the viewport's normal virtual-row
geometry, and owns neither the model, selection controller, matcher, nor
decoration collection.

`BracketNavigationController` maps Ctrl/Cmd+Shift+`\\` through the same
borrowed `LanguageBracketMatcher`, so a go-to-bracket gesture has identical
string/comment filtering to bracket highlighting. It owns only the chord,
live selection update, and reveal; pair scanning remains common-layer state.

`BracketEditingController` maps Ctrl/Cmd+Alt+Backspace to an isolated
common transaction that removes distinct valid bracket tokens. It never mutates
the hidden textarea directly and only consumes the chord if a bracket pair can
be removed.

`createCompletionWorkerFactory` is the browser-owned bridge from the
common language wire protocol to a real module `Worker`. Each factory result
owns the DOM Worker adapter and terminates its Worker on disposal. The Worker
entry owns its remote provider registry, named module registry, module host,
and wire servers. The browser requires the `language.word` module; the client
waits for its activation before sending the first completion request. Passing the factory through
`LanguageCompletionService.workerFactory` is explicit—the service's default
remains the in-process provider host. `createBrowserEditorPart`
activates this Worker path for product Aster panes.

The wire carries only versioned plain DTOs. Completion positions and ranges are
reconstructed as realm-local common values before result application.
Cancellation posts a cancel envelope, Worker errors poison the client so the
coordinator can replace it, and disposal restores all event-listener and Worker
ownership.

Protocol v4 retains the document-mirror contract: one full snapshot
initializes the Worker Piece Tree.
Subsequent model commits send ordered offset/length/text deltas, and requests
reference the mirrored version without copying document text. A version gap
falls back to full initialization; an invalid remote sync clears the mirror and
poisons the client. Immutable Piece Tree snapshots keep cancelled old-version
provider work isolated from later synchronization.

Protocol v4 additionally carries the client-confirmed per-lane result request
ID. Syntax responses may use validated ordered item splices with independent
line shifts; completion remains a full result codec. Result delta state belongs
to the common wire client/server, not the browser port adapters.

The Worker also publishes its actual provider registry through a separate
completion-catalog side channel. The browser client waits for the first
validated revision, observes later registration/removal revisions, and exposes
only immutable metadata to `LanguageCompletionService`. Custom Workers are
prewarmed so the first typed trigger can wait for this handshake. Failure clears
the old catalog; the next trigger rebuilds the Worker and waits for its new
catalog instead of routing through stale renderer metadata.

A second completion-specific side-channel publishes named provider modules and
accepts Active/Inactive requests. It never attempts to clone provider
functions. Worker-local loading and atomic registration complete before the
activation response, while the ordered port delivers the corresponding
provider-catalog revision before the browser releases its first-request
readiness barrier.

Deferred completion details use a third completion-specific side-channel.
`resolveData` never enters browser state: the renderer sends only the list
request/model/provider/item identity and receives normalized detail and
documentation text. Focus changes abort the prior request. The selected option
projects `.resolving` with `aria-busy`, then renders resolved documentation
through text nodes; resolve failure leaves the candidate list and acceptance
identity intact.

`createSyntaxWorkerFactory` is the browser-owned bridge for the shared
token/diagnostic Worker. Both lanes use one `BrowserLanguageWorkerPort`, one
incremental document mirror, and strict lane-aware result codecs. Completion
and syntax Workers reuse the same component-owned browser and dedicated
Worker port adapters, while retaining independent provider hosts and failure
domains. The Worker entry publishes `language.lexical` as a named Syntax
provider module. `SyntaxModuleWorkerClient` waits for its catalog and
successful activation before the first token or diagnostic request crosses the
shared port; module failure invalidates the prewarmed Worker so the service can
rebuild it on the next request. The Worker realm also owns one
`LanguageConfigurationRegistry` and registers the built-in ECMAScript, JSON,
and JSONC comments/brackets before loading that module. Configuration and
provider lifecycles remain independent. After each mirror sync, the common Worker host updates
`language.lexical`'s shared versioned line cache before the next ordered request,
so token and diagnostic lanes consume one incrementally computed syntax result.
The browser adapter does not own lexical state or scan policy.
`createBrowserEditorPart` consumes the Workbench `ITextMateService`,
waits for its grammar catalog, and schedules new syntax work when the catalog
changes.

Selection and decoration projection share `createAsterRangeRectangles`, so
end-exclusive ranges, selected line breaks, prefix measurement, and render-line
clipping have one implementation. Presentation sources are resolved before the
existing decoration DOM is reset; an invalid presentation therefore fails
without partially replacing the current decoration layer.

`EditorViewport.getTargetAtClientPoint` converts a PointerEvent-compatible
client point into one immutable Aster hit target without changing selection.
`Gutter`, `Text`, `EmptyContent`, and `AfterLines` remain distinct so a future
input controller can choose behavior explicitly. The browser bounds establish
viewport coordinates; fixed line height and current scroll state establish the
line, while sticky gutter geometry remains independent of horizontal scroll.

`hitTestAsterEditorPoint` resolves horizontal text positions at adjacent caret
midpoints. It uses grapheme boundaries when `Intl.Segmenter` is available and
falls back to Unicode code points, so emoji and combining sequences are not
split into arbitrary UTF-16 interiors. Prefix widths use the same
`TextMeasurer` as rendering, including tab stops and shaped text.
The grapheme implementation is owned by `editor/common/textSegmentation` and is
also consumed by keyboard navigation; browser code does not define a second
Unicode boundary policy.

`PointerSelectionController` is the first browser input policy layered on
that query. Primary-button clicks place one caret, Shift-click extends the
existing primary anchor, character drags preserve anchor/active direction, and
gutter drags select complete lines including their line breaks. Gutter
Shift-click extends the existing anchor to the selected line boundary.
Browser double-click detail selects and drags complete segments from the
common-layer `getWordSelectionRange`; triple-click detail selects and drags
complete lines. Shift-double/triple click preserves the prior primary anchor.
For a context-menu gesture, a hit inside an existing non-empty selection keeps
that selection; another hit replaces it with one caret. The controller leaves
menu composition and display to its host.

Alt+Shift primary drag is the explicit column-selection gesture. It delegates
to the common `createEditorColumnSelectionSet`, producing one same-column
selection per physical row and preserving short rows as collapsed selections;
Aster does not synthesize virtual trailing whitespace. The gesture is distinct
from the configured additive multi-cursor modifier, which never consumes Shift.

`PointerSelectionControllerOptions.multiCursorModifier` selects exact
`Alt` gestures by default or `ControlOrMeta` gestures explicitly. The selected
modifier without Shift preserves existing selections and adds the active
character, word, or whole-line result as primary. `pointerMultiCursor.ts` owns
toggle removal, last-selection retention, deduplication, overlap replacement,
and primary-index mapping. Shift keeps its ordinary extension meaning.

The adapter owns window-level move/up/cancel/blur listeners and native pointer
capture for one active drag. Its anchor is a temporary collapsed position or
complete word tracked range, so synchronous model edits during a drag cannot
leave a stale position or semantic range.
An additive gesture also tracks the original selections temporarily, preserving
their order, direction, and primary identity across the same model edits.
Pointer completion, cancellation, window blur, disposal, and failed setup all
release capture and the tracked anchor. The viewport, pointer adapter, and
selection controller reject wiring across different `TextModel` instances.

`EditorViewport.getNearestTargetAtClientPoint` clamps an active drag to
the nearest viewport edge without weakening strict `getTargetAtClientPoint`
queries. `PointerAutoScroller` maps overflow distance on each axis to a
bounded pixels-per-second velocity and uses `base/browser/AnimationFrameScheduler`
to advance the Viewport. Each frame repeats nearest-edge targeting, so the
active character, word, whole-line, or Shift selection policy continues
unchanged. Re-entry, scroll limits, completion, cancellation, blur, disposal,
and replacement by another drag stop its scheduled work.

`KeyboardNavigationController` maps local keydown events into common
character, word, line, page, and document navigation commands. It uses explicit
Windows/Linux or macOS routing, preserves Shift extension and multi-selection,
retains preferred columns across vertical movement, and ignores composing,
AltGraph, unknown, or already-handled events. Page distance comes from the
current fixed-line-height layout. `EditorViewport.revealPosition` then
uses measured line prefixes and line geometry to reveal the primary active
position horizontally and vertically. The controller deliberately does not
own text insertion, deletion, clipboard, or composition DOM events.

`TextInputController` owns a hidden textarea and redirects root focus into
it. Non-composition `beforeinput` routes ordinary text, replacement text,
Enter, Backspace, Delete, soft-line deletion, undo, and redo through the common edit-command
builders; an unmodified Tab inserts `\t`. Successful edits reveal the primary
active position. The adapter projects focus with `.input-focused`, requires the
same `TextModel` as the viewport and selection controller, and releases only
its own DOM and listeners. Composition and clipboard events are intentionally
handled separately so their platform semantics do not enter the text command
router.

Insert toggles the instance-local `.overtype` state. In that mode ordinary
single-line input replaces following graphemes without crossing a physical line
break; selected text and multiline input retain ordinary replacement behavior.
Configured bracket pairs and IME composition keep their dedicated input paths.

Optional `TextInputControllerOptions.language` binds one concrete language
ID to a caller-owned `LanguageConfigurationSource`. Opening tokens route
through common auto-close or selection-surround commands. The controller owns
one common `LanguageAutoClosingTracker` for its editor instance and records
only actions whose expected model version committed. A matching closer is
overtyped without a model transaction, and Backspace removes both sides of an
empty pair, only when that tracker confirms the closer came from Aster's
auto-close transaction. Matching user-authored text retains ordinary typing
and Backspace semantics. The configuration is resolved on each input event, so
priority changes take effect without rebuilding DOM. Browser code owns only
event adaptation and tracker lifetime; pair policy, tracked provenance,
multi-selection mapping, and history modes stay in `editor/common`.

The same language option routes `insertLineBreak` and `insertParagraph`
through `createLanguageEnterCommand`. Optional top-level `indentation` selects
tabs or spaces plus tab size for this editor instance; it is resolved before
the controller acquires DOM or tracker resources. Enter rules are fetched on
every event, so contribution priority and disposal take effect immediately.
The indentation contribution owns editor indentation policy; the language
contribution owns rule matching.

The session also derives folding from the same resolved configuration. Structural
brackets and multi-line comments remain lexical folds; `foldingMarkers` adds
language-owned named regions. Aster's built-ins recognize `// #region` and
`// #endregion` for ECMAScript, JSONC, and Rust. Marker patterns are common
configuration, so browser code only projects and toggles the resulting folding
model; a contribution must include its comment delimiter to avoid treating
ordinary source text as a region.

`EditingCommandController` owns editor-wide Select All and repeated
physical-line selection. `LineOperationsController` under
`../contrib/linesOperations/browser` also owns selected-line Tab and Shift+Tab routing.
The corresponding transformation remains DOM-free in
`../contrib/linesOperations/browser/lineIndentCommands.ts`, deduplicates physical
lines across selections, maps directional selection endpoints through the
transaction, and shares the same indentation options as
`TextInputController`. Collapsed Tab remains ordinary text input.
Word-deletion `beforeinput` types similarly route to
`common/cursor/cursorWordOperations.ts` and reuse the canonical cursor word segmentation.
Ctrl/Cmd+L expands each selection through one more physical line using the
DOM-free `../contrib/lineSelection/browser/lineSelection.ts` model; repeated use includes each non-final
line break and retains the primary selection identity.

`language.lexicalContext` may inject a caller-owned synchronous context source
for the same model/language. Without one, the controller owns a
`LanguageLexicalContextIndex` that lazily scans through the requested line and
invalidates changed suffixes. Enter uses structurally filtered slices, and
pair typing uses token identity for auto-closing `notIn`; neither command
imports browser or Worker state.

`ClipboardController` listens on that textarea for copy, cut, and paste.
It writes `text/plain`, safe preformatted `text/html`, and versioned Aster
metadata carrying text in stable selection order. When the session has a
current semantic-token source, its fixed presentation vocabulary and resolved
theme colors are included in the HTML only; unavailable or stale tokens fall
back to escaped preformatted text. Matching metadata distributes one text per
target selection; external text or invalid metadata repeats the plain text at
every selection. When `text/plain` is unavailable, inert HTML is reduced to
deterministic text without accepting script/style content. Paste and cut use
isolated common commands, then reveal the primary caret.
Platform clipboard line endings are explicit and incoming text is normalized
by the common command.

When neither text nor Aster metadata exists, one user-supplied browser `File`
may be read asynchronously and pasted as text. The adapter accepts only a
plausibly textual MIME type or known text extension up to 5 MiB; it never
opens an arbitrary local path. The same policy applies to a viewport drop at
its hit target. Completion must still match the captured model revision and
complete selection set (or the drop target revision), otherwise the result is
discarded.

`ClipboardPasteProvider` is a frontend-local extension point for declared
non-plain representations. The controller captures only immutable textual MIME
values while the native paste event is active, then gives that snapshot to
providers in declared precedence order; a provider never retains the browser
`DataTransfer`. The input controller installs `UriListPasteProvider`,
which pastes non-comment `text/uri-list` entries in source order. As with text
files, asynchronous provider output must still match the captured model version
and selections before it becomes an isolated paste command.

When a paste event contains no text, Aster metadata, recognized text file, or
matching provider, `ClipboardController` starts the browser Async
Clipboard rich reader before its plain-text fallback during the same user
gesture. Its delayed result uses the identical revision and selection gate as
text files and providers. Permission denial, an empty result, or a stale result
leaves the model unchanged. If a copy/cut event lacks `clipboardData`, the
injectable Async writer exports the same portable plain-text and safe HTML
payload; cut waits for that write to succeed before editing the model.

Cut and paste are rejected while `CompositionController` owns a protected
IME transaction; copy remains available. This prevents a browser clipboard
event from invalidating or interleaving the provisional composition revision.

Empty-selection behavior is an explicit `Line` (default) or `Ignore` policy.
Line mode copies complete lines with LF ownership, merges duplicate/overlapping
cut ranges, and writes version 2 metadata. A matching all-line paste inserts at
target line starts while preserving columns; multiple carets on one target
line become one ordered insertion. Mixed modes or non-empty targets fall back
to ordinary per-selection paste. Plain line entries concatenate without
introducing extra blank separators.

`CompositionController` maps a desktop-style composition event sequence
to one protected common session. It reads textarea-relative selection when the
value matches the event's complete provisional text, otherwise placing the
caret at the normalized text end. Escape, blur, disposal, and `IME.disable()`
cancel; end commits. Direct model observation detects external invalidation
even without a selection movement. The controller publishes active state,
projects `.composing`/`.ime-input`, and positions the textarea through
`EditorViewport.getPositionContentCoordinates` so the native candidate
window follows the measured caret and line height.

The common session exposes its active `currentRange`; the Viewport owns a
temporary tracked copy and projects it through the same range geometry used by
selections and decorations. A separate composition layer renders
component-owned underline segments across visible lines, including newline cells. It is
cleared on commit, cancellation, external invalidation, IME disable, and
disposal. Empty active end commits deletion, while an end arriving after the
session closed is ignored.

The component reuses `base/browser/dom` for disposable events and DOM reset,
`base/browser/geometry` for host measurement, and `base/common/lifecycle` for
ownership. Its `.aster-editor` root and internal line classes are styled
only by `media/editorViewport.css`; Workbench hosts may size the root and
explicitly select `focusOutlineOwner: "host"` when their direct control owns
one surrounding focus indicator, but must not override internal rows or focus
state. The component projects stable
`.active` state for the primary active line and `.primary` identity on the
primary caret; CSS does not use ARIA attributes as visual selectors. Embedded
presentations omit `.active` by default. Vertical padding participates in the
viewport content height and row coordinates instead of being simulated by a
host selector; left and right padding are component-owned text-measurement
inputs, so wrapping, carets, placeholders, and pointer hits share one inset.

`DomTextMeasurer` derives the active font, letter spacing, tab size, and
horizontal padding from the line layer's computed style. Canvas measures
shaped segments and fallback glyphs; tabs advance to measured space-based tab
stops. `TextMeasurer` is the small injectable contract used by tests and
future specialized font engines.

`LineWidthIndex` takes one bounded synchronous first slice when the view
is constructed, then schedules remaining non-wrapped lines through cancellable
idle slices. Its current maximum is a lower bound until completion, never a
claim about unmeasured lines. An edit cancels a pending generation and starts
again from the current model version. After completion, transactions group edits
by affected old line ranges, measure only their new line groups, and update a
counted width set. Font refreshes restart this same policy because every cached
width may have changed. Browser font-loading completion and explicit
`refreshFontMetrics()` both trigger it.

The current renderer is fixed-line-height and mutates text only through the
hidden textarea adapter, never through contenteditable DOM. Focused carets
blink through component-owned CSS when motion is allowed and remain visible
when the user requests reduced motion.
Canvas-unavailable runtimes use a font-size-derived fallback advance. For wrapped
documents, `VisualLineProjection` synchronously measures only a bounded
first line slice, leaves later logical lines as one-row placeholders, and
finishes their wrap measurement through cancellable idle slices before atomically
publishing the complete projection. Parser-grade folding ranges,
IME clause segmentation and mobile composition variants,
semantic-token delta presentation beyond Aster's closed modifier vocabulary,
TextMate extension-resource discovery, native browser-driven
wrapping, and empirical cross-platform screen-reader acceptance remain future
work. Mixed-run BiDi uses `domTextGeometry.ts` to obtain browser `Range`
rectangles and caret positions for visible selection/caret, decorations,
composition anchors, pointer hits, and vertical navigation. The macOS desktop
DOM contract uses one accessible textarea for IME and VoiceOver, sharing its
explicit `dir` with the rendered projection.
`SemanticTokenModifier` maps only standard declaration/readonly/static/
deprecated/abstract/async names to component-owned classes; unknown provider
strings are intentionally excluded from DOM projection. Syntax result deltas and relative token-line
payload reuse are common-layer capabilities; this browser layer consumes only
the current visible-line query and does not own their persistence policy.
Visible clipping uses `DecorationLineIndex` to resolve only decorations whose logical-line
range intersects the rendered window; the index is rebuilt only when a source collection changes.
