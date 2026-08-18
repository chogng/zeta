# Browser foundation

`src/zeta/base/browser` contains browser-runtime capabilities shared by UI,
platform, and workbench code. It intentionally does not provide a universal
DOM component base class.

## Dependency direction

```text
base/common
    -> base/browser
    -> base/browser/ui
    -> platform/browser
    -> workbench/browser
```

Browser foundation modules may depend on `base/common`, but must not import
from UI, platform, or workbench modules.

## Modules

| Module | Responsibility |
| --- | --- |
| `dom.ts` | Disposable native listeners and small DOM primitives |
| `window.ts` | Main/auxiliary window identity, registration, and lookup |
| `focus.ts` | Active-element lookup, tracking, restoration, Tab order, and focus containment |
| `geometry.ts` | DOM dimensions and viewport/page coordinate measurement |
| `../common/layout.ts` | Pure, DOM-independent anchored layout calculation |
| `observer.ts` | Disposable Resize, Mutation, and Intersection observers |
| `scheduler.ts` | Idle work, animation-frame coalescing, measure/modify order |
| `keyboardEvent.ts` | Stable keyboard-event representation |
| `../common/keybindings.ts` | Logical/physical chords, sequences, and OS resolution |
| `../common/keybindingParser.ts` | External keybinding string parsing |
| `../common/keybindingLabels.ts` | UI, ARIA, and user-settings labels |
| `../common/ime.ts` | IME enablement coordination during chord dispatch |
| `mouseEvent.ts` | Stable mouse and pointer coordinates across windows |
| `dnd.ts` | Drag depth and DataTransfer helpers |
| `fileAccess.ts` | Browser file picking, object URLs, and downloads |
| `fullscreen.ts` | Fullscreen state and lifecycle |
| `domBuilder.ts` | Explicit-Document HTML and SVG construction |
| `reactiveDom.ts` | Bindings for snapshot-plus-event readable state |
| `domStylesheets.ts` | Disposable and multi-window dynamic stylesheets |
| `aria.ts` | Per-document ARIA live announcements |

## Overlay boundaries

- `common/layout.ts` calculates placement without importing browser APIs.
- `ui/contextview` owns overlay attachment, dismissal, focus restoration, and
  applying the calculated coordinates.
- `ui/hover`, `ui/dropdown`, and `ui/selectbox` own their interaction and ARIA
  semantics; they do not add component-specific policy to `geometry.ts`.

## Drag-and-drop boundaries

- `base/browser/dnd.ts` owns native listener normalization and browser
  `DataTransfer` helpers. It does not coordinate collection state or product
  payloads.
- `base/browser/ui/dnd` defines domain-neutral drag origins and shared visual
  state. `base/browser/ui/list/listView.ts` owns flat rows, sizing, scrolling,
  the canonical drag session, cross-list transfer, target sectors, and
  feedback. `listWidget.ts` owns selection, focus, keyboard, and pointer
  semantics over that View.
- Tree controls adapt that List contract to model nodes. Tree alone owns
  hierarchical bubbling, subtree feedback, and delayed expansion.
- `platform/dnd` retains typed same-renderer payload identity. Workbench
  consumers own Editor, View, file, and other product mutation semantics.
- Action bars and tabs may keep their collection-specific insertion geometry;
  they are not forced through the vertical List controller. Do not introduce a
  global DnD manager that takes semantic drop policy away from components.

## Focus architecture

The long-term focus model is **window-level coordination, scope-level
execution, and component-level decisions**.

- Window-level coordination determines which registered window or external
  surface currently owns application focus. It does not navigate widgets.
- `focus.ts` provides mechanisms: active-element lookup across open shadow
  roots, focus-within tracking, tabbable order, safe restoration, and reusable
  focus movement.
- A local focus scope, such as a dialog or context view, executes declared
  initial-focus, containment, and restoration policies for its own lifetime.
- Components retain semantic decisions. Action bars interpret horizontal
  arrows, menus interpret vertical arrows, select boxes own their active
  option, and tooltips do not receive focus.

Do not introduce a global focus manager that assigns focus inside arbitrary
components. Extract a reusable roving-focus controller only after multiple
components share the same navigation contract.

## Keyboard event boundaries

Keyboard events support focus policy but do not belong to the focus model.

- Local widget behavior uses the native `KeyboardEvent`. Tab, Escape, Enter,
  Home/End, and arrow navigation are semantic keys and should be compared
  through `event.key`.
- `keyboardEvent.ts` normalizes a native event at the document-level dispatch
  boundary. It preserves both the layout-aware `key` and physical `code`.
- `common/keybindings.ts` owns the DOM-independent model. Logical chords match
  `event.key`; physical chords match `event.code`; `primaryKey` resolves to
  Command on macOS and Control elsewhere.
- `platform/keybinding/common` owns contribution registration, conflict
  priority, multi-chord resolution, and ContextKey conditions.
- `platform/keyboardLayout/common` defines the active layout and mapper
  contract without importing browser APIs.
- `workbench/services/keybinding/browser/keyboardLayoutService.ts` uses the
  browser Keyboard Map capability when available and otherwise preserves a
  stable physical-code fallback.
- `workbench/services/keybinding/browser/keybindingService.ts` is the concrete
  product service. It owns document listeners, chooses the nearest DOM
  ContextKey scope, reports chord state, prevents handled native events, and
  invokes commands.
- `Action2` may contribute a primary and secondary keybinding, but remains
  independent of its final menu, toolbar, or keyboard presentation.
- `keybindingParser.ts` is an input boundary for user or extension strings.
  Built-in contributions use typed `Keybinding` objects and do not parse
  strings during registration.
- `keybindingLabels.ts` formats resolved bindings. The browser
  `KeybindingLabel` only renders those results and does not resolve shortcuts.
- A component is not required to construct `StandardKeyboardEvent` merely to
  inspect one local semantic key.
- Ignore shortcut and type-ahead activation while `event.isComposing` is true,
  so IME composition receives the first opportunity to handle the event.
- Do not interpret AltGraph as a Ctrl+Alt shortcut.
- Call `preventDefault` or stop propagation only after a component has
  actually handled the event.
- Entering a multi-chord wait state temporarily disables the shared IME state.
  Text inputs observe that state and suppress composition until the chord
  resolves, times out, or loses window focus.
- Chord and composition state are published through
  `keybinding.inChordMode` and `keybinding.isComposing`; the status bar exposes
  the pending chord without moving that product policy into platform code.
- Persisted keybindings are an ordered resource independent of ordinary
  configuration. `IKeybindingsResourceService` projects the active
  `keybindings.json` into user-weight resolver rules.
- A user entry requires `{ key, command }` and may define `when`, `args`,
  `mac`, `linux`, and `win`. A platform override set to `null` disables that
  rule on the platform.
- `command: null` installs an explicit blocker at user weight, removing both
  dispatch and displayed shortcut lookup for the lower-priority binding.

## Context key scopes

Context keys connect focus-local state to actions, menus, and keybindings.

- The root service stores window-wide values.
- `createScoped(element)` creates an inheriting context for that DOM subtree.
- Event dispatch resolves the nearest scoped service from the composed target.
- `RawContextKey<T>` provides typed binding and default reset behavior.
- Components decide which semantic values they publish; the context service
  only stores, inherits, and evaluates them.
- Persisted `when` strings are parsed at the user-resource boundary. Built-in
  code continues to compose typed `ContextKeyExpr` values.

## Context menu architecture

- `platform/contextview` defines `IContextMenuService` and owns reusable HTML
  and native rendering mechanisms.
- `workbench/services/contextmenu` is the product boundary. It combines menu
  actions with resolved keybindings, owns the selected implementation, and
  applies browser or Electron host policy.
- Browser hosts select the HTML implementation. Electron hosts use the native
  implementation on macOS and the HTML implementation on Windows and Linux.
- Consumers depend only on `IContextMenuService`; they do not choose a
  renderer or access the Electron bridge.
- The service identifier remains in `platform/contextview`. A workbench
  service is a concrete product implementation, not a second contract.

## Configuration architecture

Configuration, application state, and Rust product intent are separate
authorities:

- `platform/configuration/common` defines typed configuration keys, the
  `IConfigurationService` contract, and the bounded versioned wire document.
- `workbench/services/configuration` validates host snapshots through the
  registered keys and publishes atomic changes to product services.
- Electron Main owns `<profile>/configuration.json`, performs atomic writes, watches for
  external edits, and enforces compare-and-swap revisions. Renderer access is
  restricted to the typed read/update/change preload capability.
- Electron Main independently owns `<profile>/keybindings.json` under the same
  revisioned JSON storage primitive. This preserves ordered shortcut rules
  without turning them into an ordinary configuration value.
- Browser hosts use the same Workbench service with an in-memory document
  until a browser persistence host is supplied.
- `configuration.json` stores frontend/device key/value settings such as menu
  presentation, fonts, accessibility, and product theme selections. Keyboard
  shortcuts belong to `keybindings.json`.
- `state.json` stores reconstructable machine state such as window bounds. It
  must not become a configuration store.
- The Rust ConfigStore remains authoritative for cross-client backend intent
  such as models, providers, MCP servers, Skills, and Workspace trust.
  Presentation preferences, keyboard events, and Desktop command IDs do not
  cross that boundary.

The current Desktop document is:

```json
{
  "version": 1,
  "values": {
    "editor.fontSize": 14
  }
}
```

Configuration keys are declared once through `ConfigurationsRegistry`.
Consumers request values with the returned typed key instead of repeating
string addresses or casting untrusted persisted values.

The active `keybindings.json` is a top-level ordered array:

```json
[
  {
    "key": "primary+n",
    "command": "zeta.startTurn",
    "when": "windowFocused && !inputFocus",
    "args": {
      "source": "keyboard"
    },
    "mac": "cmd+n",
    "win": "ctrl+n"
  },
  {
    "key": "primary+shift+n",
    "command": null
  }
]
```

The host capability represents the active keybinding resource rather than a
fixed path. A future profile service can switch that resource without changing
the resolver, contribution, or command layers.

## Design rules

- Pass or derive the owning `Document` instead of assuming the global
  `document`.
- Resolve a `Window` from its node or document before registering global
  listeners or timers.
- Every listener, observer, scheduler, and temporary URL returns or owns an
  `IDisposable`.
- Keep one-line native DOM operations native unless a wrapper enforces
  lifecycle, cross-window, compatibility, or accessibility semantics.
- Leaf action representations render into a host-owned container. Layout and
  workbench views may expose a structural `element` without sharing a concrete
  DOM base class.
- Reactive DOM consumes the small `ReadableValue` protocol. It does not create
  a global event bus or require a framework-specific observable graph.
