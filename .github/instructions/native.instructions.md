---
description: Zeta native UI ownership and the zeta-rs/native deprecation boundary.
applyTo: "zeta-rs/native/**,app/**"
---

# Native UI Ownership Guidelines

`zeta-rs/native` is in deprecation migration. Do not add product capability, reusable UI machinery, components, layout algorithms, interaction trees, animation, deadlines, retained lifecycle, registries, timers, or new state owners there.

Route new capability to its long-term owner:

- backend-independent frame, layout, paint, inspection, interaction, animation, invalidation, and retained lifecycle contracts belong in `app/zui`;
- reusable application/window lifecycle, renderer initialization, platform capability, event-loop, and multi-window orchestration belong to the single public `app/zui` crate; its foundation/layout/text/presentation/runtime/application/platform/renderer modules are private implementation boundaries, not sibling crates or alternative entry points;
- reusable UI controls belong in `app/ui-components` (`zeta-ui-components`), while Workbench titlebar, tab navigation, interaction identities, and presentation state belong in `app/workbench-ui` (`zeta-workbench-ui`); generic layout algorithms remain in `app/zui`;
- file, SCM, editor, terminal, and other domain behavior belongs in its domain crate;
- `app` owns product state mapping, product event meaning, scene construction, and the native product entry point; it consumes platform events and rendering only through public `zui` contracts.

Changes in `zeta-rs/native` are limited to compatibility needed to remove or migrate old implementation, thin mapping from platform/product state into canonical lower APIs, and minimal wiring required to keep the existing host running. An exception must name the long-term owner, migration endpoint, and deletion condition.

When modifying existing native files, prefer migrating, deleting, deprecating, or narrowing responsibility. Historical split scene/interaction hosting is debt, not a foundation for new helpers or public abstractions.
