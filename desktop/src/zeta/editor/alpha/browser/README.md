# Alpha Editor browser layer

This directory owns Alpha's native browser projection of contracts from
`../common/`. It may import `base/common`, `base/browser`, and `alpha/common`.
Neither `alpha/common` nor `base` may import this layer.

Browser-owned responsibilities include:

- viewport observation and DOM virtualization;
- textarea, keyboard, clipboard, drag/drop, and composition-event adapters;
- font and glyph measurement;
- DOM selection, focus, scrolling, and pointer projection;
- ARIA and screen-reader surfaces;
- presentation of editor-owned decorations and semantic tokens.

Pure text, history, selection, decoration identity, composition transaction
semantics, and DOM-independent layout math remain in `alpha/common`.
Workbench parts may host the browser editor and own its external box, but must
not reach through its component internals. Visual rules follow
[`docs/ui-styling-ownership.md`](../../../../../../docs/ui-styling-ownership.md).

Monaco and ProseMirror are sibling transition adapters. Native browser code
must not import them or expose their types through Zeta contracts.

## Current implementation

`AlphaEditorViewport` is the first native browser projection. It consumes
`EditorViewportModel`, creates one component-owned scroll surface, and renders
only the overscanned line range. Overlapping lines retain their DOM identity
while scrolling. A new model version updates visible row text synchronously,
and a shrinking document clamps both the common viewport state and native DOM
scroll coordinates.

Each virtual row owns a sticky line-number gutter, a text node, and a
presentation overlay. Gutter width is derived from the current line-count digit
width and participates in authoritative horizontal content measurement.
`AlphaEditorViewport` may observe one `EditorSelectionController`; it projects
the controller's primary active line through `.active`, multi-line selected
ranges as overlay rectangles, and every active edge as a caret. The controller
remains caller-owned.

`createAlphaSelectionGeometry` is the DOM-independent browser geometry seam.
It preserves anchor/active direction and primary-selection identity while
clipping output to the current render range. Horizontal positions are measured
from line prefixes using the active `AlphaTextMeasurer`. A selected line break
occupies one measured space cell so an end-exclusive multi-line range remains
visible even when the following endpoint is at column zero.

`createAlphaDecorationSource` adapts one caller-owned
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
`createAlphaLanguageDiagnosticSource` then maps Error and Warning to the
component's existing named underlines. Information and Hint stay in common
metadata and are intentionally omitted from DOM until dedicated presentations
and diagnostic theme tokens exist. The viewport owns neither bridge, collection,
store, nor model.

`createAlphaSemanticTokenSource` adapts one caller-owned
`LanguageTokenLineIndex` to Alpha's closed browser presentation vocabulary.
Unknown worker token types are omitted by default; custom language resolvers
may inspect token modifiers but must still return a named Alpha presentation,
never an arbitrary CSS class. `getLineTokens` resolves one indexed line on
demand. The viewport resolves only its target virtual range before replacing
any row, so offscreen lines do not invoke the resolver and a resolver failure
cannot partially update the visible window.

`projectAlphaSemanticTokenLine` validates each line snapshot before mutation,
then creates text nodes and component-owned spans without parsing HTML. Joined
DOM text must equal the model line exactly. Same-version token replacement
rerenders visible line content without rebuilding rows; model invalidation
removes stale spans before the normal model-version projection. The viewport
owns only its event registration, not the source, common index, result store,
or model. Semantic foregrounds come from registered
`editor.token.*Foreground` theme colors owned by editor presentation. Legacy
`editor.semanticToken.*Foreground` overrides are normalized only at the user-theme boundary.

`AlphaCompletionWidget` projects one caller-owned
`LanguageCompletionSessionController` below the viewport's measured trigger
position. It creates only component-owned listbox/option DOM, maps completion
kinds through a closed text vocabulary, and projects `.visible` and `.focused`
alongside ARIA state. The hidden textarea intercepts ArrowUp/ArrowDown,
Enter/Tab, and Escape only while a session is active. Mouse acceptance happens
on mousedown so the synchronous focus rerender cannot detach the target before
the edit is accepted.

`AlphaTextInputControllerOptions.completion` opts into this widget through its
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
`media/completionWidget.css` and consume existing dialog, list-selection,
foreground, border, and shadow theme tokens.

`createAlphaCompletionWorkerFactory` is the browser-owned bridge from the
common language wire protocol to a real module `Worker`. Each factory result
owns the DOM Worker adapter and terminates its Worker on disposal. The Worker
entry owns its remote provider registry, named module registry, module host,
and wire servers. The browser requires the `alpha.word` module; the client
waits for its activation before sending the first completion request. Passing the factory through
`LanguageCompletionService.workerFactory` is explicit—the service's default
remains the in-process provider host. `createBrowserAlphaEditorSession`
activates this Worker path for product Alpha panes.

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
ID. Analysis responses may use validated ordered item splices with independent
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

`createAlphaAnalysisWorkerFactory` is the browser-owned bridge for the shared
token/diagnostic Worker. Both lanes use one `BrowserLanguageWorkerPort`, one
incremental document mirror, and strict lane-aware result codecs. Completion
and analysis Workers reuse the same component-owned browser and dedicated
Worker port adapters, while retaining independent provider hosts and failure
domains. The Worker entry publishes `alpha.lexical` as a named Analysis
provider module. `LanguageAnalysisModuleWorkerClient` waits for its catalog and
successful activation before the first token or diagnostic request crosses the
shared port; module failure invalidates the prewarmed Worker so the service can
rebuild it on the next request. The Worker realm also owns one
`LanguageConfigurationRegistry` and registers the built-in ECMAScript, JSON,
and JSONC comments/brackets before loading that module. Configuration and
provider lifecycles remain independent. After each mirror sync, the common Worker host updates
`alpha.lexical`'s shared versioned line cache before the next ordered request,
so token and diagnostic lanes consume one incrementally computed analysis.
The browser adapter does not own lexical state or scan policy.
`createBrowserAlphaEditorSession` selects the TextMate analysis factory, waits
for its grammar catalog, and schedules new analysis when the catalog changes.

Selection and decoration projection share `createAlphaRangeRectangles`, so
end-exclusive ranges, selected line breaks, prefix measurement, and render-line
clipping have one implementation. Presentation sources are resolved before the
existing decoration DOM is reset; an invalid presentation therefore fails
without partially replacing the current decoration layer.

`AlphaEditorViewport.getTargetAtClientPoint` converts a PointerEvent-compatible
client point into one immutable Alpha hit target without changing selection.
`Gutter`, `Text`, `EmptyContent`, and `AfterLines` remain distinct so a future
input controller can choose behavior explicitly. The browser bounds establish
viewport coordinates; fixed line height and current scroll state establish the
line, while sticky gutter geometry remains independent of horizontal scroll.

`hitTestAlphaEditorPoint` resolves horizontal text positions at adjacent caret
midpoints. It uses grapheme boundaries when `Intl.Segmenter` is available and
falls back to Unicode code points, so emoji and combining sequences are not
split into arbitrary UTF-16 interiors. Prefix widths use the same
`AlphaTextMeasurer` as rendering, including tab stops and shaped text.
The grapheme implementation is owned by `alpha/common/textSegmentation` and is
also consumed by keyboard navigation; browser code does not define a second
Unicode boundary policy.

`AlphaPointerSelectionController` is the first browser input policy layered on
that query. Primary-button clicks place one caret, Shift-click extends the
existing primary anchor, character drags preserve anchor/active direction, and
gutter drags select complete lines including their line breaks. Gutter
Shift-click extends the existing anchor to the selected line boundary.
Browser double-click detail selects and drags complete segments from the
common-layer `getWordSelectionRange`; triple-click detail selects and drags
complete lines. Shift-double/triple click preserves the prior primary anchor.

`AlphaPointerSelectionControllerOptions.multiCursorModifier` selects exact
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

`AlphaEditorViewport.getNearestTargetAtClientPoint` clamps an active drag to
the nearest viewport edge without weakening strict `getTargetAtClientPoint`
queries. `AlphaPointerAutoScroller` maps overflow distance on each axis to a
bounded pixels-per-second velocity and uses `base/browser/AnimationFrameScheduler`
to advance the Viewport. Each frame repeats nearest-edge targeting, so the
active character, word, whole-line, or Shift selection policy continues
unchanged. Re-entry, scroll limits, completion, cancellation, blur, disposal,
and replacement by another drag stop its scheduled work.

`AlphaKeyboardNavigationController` maps local keydown events into common
character, word, line, page, and document navigation commands. It uses explicit
Windows/Linux or macOS routing, preserves Shift extension and multi-selection,
retains preferred columns across vertical movement, and ignores composing,
AltGraph, unknown, or already-handled events. Page distance comes from the
current fixed-line-height layout. `AlphaEditorViewport.revealPosition` then
uses measured line prefixes and line geometry to reveal the primary active
position horizontally and vertically. The controller deliberately does not
own text insertion, deletion, clipboard, or composition DOM events.

`AlphaTextInputController` owns a hidden textarea and redirects root focus into
it. Non-composition `beforeinput` routes ordinary text, replacement text,
Enter, Backspace, Delete, undo, and redo through the common edit-command
builders; an unmodified Tab inserts `\t`. Successful edits reveal the primary
active position. The adapter projects focus with `.input-focused`, requires the
same `TextModel` as the viewport and selection controller, and releases only
its own DOM and listeners. Composition and clipboard events are intentionally
handled separately so their platform semantics do not enter the text command
router.

Optional `AlphaTextInputControllerOptions.language` binds one concrete language
ID to a caller-owned `LanguageConfigurationSource`. Opening tokens route
through common auto-close or selection-surround commands. The controller owns
one common `LanguageAutoClosingTracker` for its editor instance and records
only actions whose expected model version committed. A matching closer is
overtyped without a model transaction, and Backspace removes both sides of an
empty pair, only when that tracker confirms the closer came from Alpha's
auto-close transaction. Matching user-authored text retains ordinary typing
and Backspace semantics. The configuration is resolved on each input event, so
priority changes take effect without rebuilding DOM. Browser code owns only
event adaptation and tracker lifetime; pair policy, tracked provenance,
multi-selection mapping, and history modes stay in `alpha/common`.

The same language option routes `insertLineBreak` and `insertParagraph`
through `createLanguageEnterCommand`. Optional top-level `indentation` selects
tabs or spaces plus tab size for this editor instance; it is resolved before
the controller acquires DOM or tracker resources. Enter rules are fetched on
every event, so contribution priority and disposal take effect immediately.
The browser owns neither indentation policy nor rule matching.

`language.lexicalContext` may inject a caller-owned synchronous context source
for the same model/language. Without one, the controller owns a
`LanguageLexicalContextIndex` that lazily scans through the requested line and
invalidates changed suffixes. Enter uses structurally filtered slices, and
pair typing uses token identity for auto-closing `notIn`; neither command
imports browser or Worker state.

`AlphaClipboardController` listens on that textarea for copy, cut, and paste.
It writes `text/plain` plus versioned Alpha metadata carrying text in stable
selection order. Matching metadata distributes one text per target selection;
external text or invalid metadata repeats the plain text at every selection.
Paste and cut use isolated common commands, then reveal the primary caret.
Platform clipboard line endings are explicit and incoming text is normalized
by the common command.

Empty-selection behavior is an explicit `Line` (default) or `Ignore` policy.
Line mode copies complete lines with LF ownership, merges duplicate/overlapping
cut ranges, and writes version 2 metadata. A matching all-line paste inserts at
target line starts while preserving columns; multiple carets on one target
line become one ordered insertion. Mixed modes or non-empty targets fall back
to ordinary per-selection paste. Plain line entries concatenate without
introducing extra blank separators.

`AlphaCompositionController` maps a desktop-style composition event sequence
to one protected common session. It reads textarea-relative selection when the
value matches the event's complete provisional text, otherwise placing the
caret at the normalized text end. Escape, blur, disposal, and `IME.disable()`
cancel; end commits. Direct model observation detects external invalidation
even without a selection movement. The controller publishes active state,
projects `.composing`/`.ime-input`, and positions the textarea through
`AlphaEditorViewport.getPositionContentCoordinates` so the native candidate
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
ownership. Its `.zeta-alpha-editor` root and internal line classes are styled
only by `media/alphaEditorViewport.css`; Workbench hosts may size the root but
must not override internal rows or focus state. The component projects stable
`.active` state for the primary active line and `.primary` identity on the
primary caret; CSS does not use ARIA attributes as visual selectors.

`AlphaDomTextMeasurer` derives the active font, letter spacing, tab size, and
horizontal padding from the line layer's computed style. Canvas measures
shaped segments and fallback glyphs; tabs advance to measured space-based tab
stops. `AlphaTextMeasurer` is the small injectable contract used by tests and
future specialized font engines.

`AlphaLineWidthIndex` scans every line once when the view is constructed. Model
transactions then group edits by affected old line ranges, measure only their
new line groups, and update a counted width set. This keeps the maximum content
width exact for the active measurer without rescanning the whole document after
ordinary typing. Font refreshes intentionally rebuild the complete index
because every cached width may have changed. Browser font-loading completion
and explicit `refreshFontMetrics()` both trigger this path.

The current renderer is fixed-line-height and mutates text only through the
hidden textarea adapter, never through contenteditable DOM.
The initial width scan is synchronous, and Canvas-unavailable runtimes use a
font-size-derived fallback advance. Very large documents may require chunked
initial measurement later. Soft wrapping, folding, rich clipboard, file paste,
Android/macOS/iOS composition variants, dead keys,
IME clause segmentation, caret blinking, rich decoration hover/overview-ruler
behavior, Information/Hint diagnostic presentation, semantic-token delta
modifier styling, TextMate grammar integration, and complete screen-reader
semantics remain unimplemented. Analysis result deltas and relative token-line
payload reuse are common-layer capabilities; this browser layer consumes only
the current visible-line query and does not own their persistence policy.
Visible clipping currently scans every resolved decoration on each layout;
very large collections will require a line-range index before product adoption.
