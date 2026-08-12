# Code Sessions

`sessions/` owns Zeta Code's dedicated agent Workbench beside the regular
`workbench/`. Its product and build boundary is canonical in
[`docs/product-editions.md`](../../../../docs/product-editions.md); this README
is canonical for the renderer implementation and extension points.

## Ownership

| Area | Owner | Current implementation |
| --- | --- | --- |
| Window host | `browser/sessionsWorkbench.ts` | validates the Code profile, binds theme tokens, and creates one Sessions runtime |
| Product composition | `browser/code/codeSessionsWorkbench.ts` | composes the fixed titlebar/sidebar/sessions/auxiliarybar Part set |
| Layout | `browser/layout/` and `services/layout/` | owns Sessions-only topology, geometry, optional auxiliary visibility, and persisted sizes |
| Window view state | `services/view/` | projects the canonical Session model into active/visible selections and Back/Forward history |
| Session model | `workbench/services/sessions/` | owns App Server Session/Thread state, mutation commands, subscriptions, and authoritative refresh |
| Main conversation | `browser/common/sessionsChatView.ts` | renders visible durable and untitled Sessions as retained full `ChatPane` Grid leaves |
| Parts | `browser/parts/` | owns product chrome, list, primary surface, and typed active context |

The dependency direction is one-way: Sessions may reuse backend-neutral
Workbench mechanisms and contributions; regular Workbench modules must not
import `sessions/` or add Sessions-specific branches to `WorkbenchLayout`.

## Execution path

1. The Code browser or Electron entry calls `startSessionsWorkbench`.
2. `SessionsRuntime` creates one `WorkbenchSessionService`,
   `SessionsViewService`, and `ChatService`, and registers their frontend
   contracts in a window-local `ServiceCollection`.
3. `CodeSessionsWorkbench` creates `BrowserLayoutService` and the shared
   `WorkbenchInteractionServices`, so Chat uses the same commands, context
   keys, menus, keybindings, overlays, quick input, settings, and hover
   mechanisms as the regular Workbench.
4. `SessionsWorkbenchLayout` deserializes the fixed Part grid. Titlebar,
   sidebar, and sessions Parts are required; only the auxiliary Part may hide.
5. `SessionsViewService.initialize` loads canonical Sessions. If none is
   active, the runtime opens a window-local untitled Session; it becomes
   durable only when the first message is sent.
6. `SessionsChatView` reconciles every visible selection with a retained
   `ChatPane` leaf in an internal, resizable `Grid`. Focus projects the leaf
   back to the active selection; closing a leaf does not archive its durable
   Session, and draft materialization preserves the leaf in place.
   The product composition owns the single view-service subscription and
   pushes `(visible, active)` into the passive `SessionsPart`.

App Server `session/update` notifications are sequence hints, not frontend
state. `WorkbenchSessionService` subscribes to active Sessions, re-subscribes
from its current sequence when a newer sequence is announced, applies the
authoritative aggregate snapshot returned with the durable gap, updates active
selection references, and unsubscribes when a Session is stopped or archived.
A snapshot that cannot advance to the announced sequence fails explicitly
instead of entering a refresh loop.

## Failure and lifecycle semantics

- The renderer never creates a durable Session merely because the window or a
  draft opens.
- A catalog load failure still permits a window-local draft; the first send
  reports any backend failure when durable Session materialization is needed.
- Session and Thread mutations remain App Server-owned; the view service owns
  only window-local visibility and navigation history.
- Each runtime, Part, retained Chat pane, App Server event subscription, and
  interaction service is disposed with the Sessions window.
- Returning to Workbench closes the Electron Sessions window or navigates the
  browser page to its sibling Workbench entry.
- Academic currently has no dedicated Sessions renderer or profile.

## Tests and modification impact

- `test/browser/sessions-layout.test.ts` protects fixed topology, required
  Parts, and optional auxiliary visibility.
- `test/browser/sessions-view-service.test.ts` protects selection ownership,
  multi-session visibility, history, stale references, close behavior, and
  draft materialization.
- `test/browser/sessions-part.test.ts` verifies the Sessions-owned primary Part
  passively renders multiple full Chat surfaces and reports focus/close intent.
- `workbench/services/sessions/test/browser/sessionService.test.ts` protects
  subscription, authoritative live refresh, active projection, and stale
  snapshot failure behavior.
- `test/smoke/areas/sessions/sessions-window.spec.ts` verifies the dedicated
  Electron window, all four Parts, multiple Grid leaves, close, and return flow.

Changes to topology belong in `browser/layout/`; changes to window selection
belong in `services/view/`; changes to canonical Session persistence or update
semantics belong in `workbench/services/sessions/`. Adding a second layout or
Session model inside a Part would be architectural drift.

## Current limitations and staged evolution

The Sessions Part currently arranges visible Session leaves in one horizontal
Grid row. Two-dimensional placement persistence, provider grouping,
search/filtering, archived history, and cross-window view-state persistence are
future work. They should extend the Sessions view/layout owners without moving
product policy into base modules or the regular Workbench layout.
