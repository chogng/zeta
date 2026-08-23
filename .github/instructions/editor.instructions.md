---
description: Zeta editor ownership, projection, input, layout, and rendering rules.
applyTo: "**/src/zeta/editor/**/*.ts,**/test/editor/**"
---

# Editor Implementation Guidelines

See [`src/zeta/editor/README.md`](../../desktop/src/zeta/editor/README.md) and [`text-engine-architecture.md`](../../desktop/src/zeta/editor/text-engine-architecture.md).

## Ownership

- Give each editor state one canonical owner. Browser code retains only DOM and measurement state needed for projection.
- Keep the text model and structured document model as independent mutation authorities.
- Contributions depend on engine contracts; they do not introduce product IDs, duplicate model state, or import product bundles.

## View parts and DOM

- The object that creates stable DOM owns its lifecycle.
- Retain stable nodes when reuse is cheap. Repeated rendering must not leak nodes, listeners, canvases, or disposables.
- A Part writes only its own DOM and consumes host layout; it does not become another viewport owner.

## Context and dependencies

- Keep shared view context limited to current layout and canonical view-model, scheduling, measurement, and projection access.
- Centralize overlay snapshot creation and version validation in the view context.
- Keep feature state with its owner; decoration consumers depend directly on `DecorationsPart` rather than accessing it through the context.
- Pass feature dependencies directly. Do not turn context into a service locator.

## Update, layout, and rendering

- Layout owns geometry and must not mutate editor model state.
- Render projects owned state into DOM or canvas. It is synchronous and does not mutate models or register listeners.
- Add `prepareRender` only when the scheduler separates measurement and mutation phases.
- Avoid layout-forcing DOM reads after writes in the same render phase.
- Keep short render algorithms together. Extract only complex, shared, or independently invalidated work.

## Projection and hit testing

- Projection converts authoritative state into geometry or presentation; it does not own DOM lifecycle.
- Hit tests return semantic targets. Controllers decide selection, scrolling, and commands.
- Use one version check and one coordinate conversion per operation. Share logical-line and visual-line hit-test flow after normalization.

## Controllers and events

- An input controller rejects unhandled events, resolves one intent, and invokes the owning state transition.
- Do not combine key mapping, selection algorithms, cleanup, announcements, and command execution in one event method.
- Events report state changes; direct calls drive synchronous control flow.

## Public API and performance

- Keep common editor contracts independent of browser and Workbench services.
- Require explicit invalidation for every cache or retained projection.
- Measure before adding per-Part DOM write caches.
