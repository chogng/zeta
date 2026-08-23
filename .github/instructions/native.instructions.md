---
description: Zeta native UI ownership and the zeta-rs/native deprecation boundary.
applyTo: "zeta-rs/native/**,zeterm/**"
---

# Native UI Ownership Guidelines

`zeta-rs/native` is in deprecation migration. Do not add product capability, reusable UI machinery, components, layout algorithms, interaction trees, animation, deadlines, retained lifecycle, registries, timers, or new state owners there.

Route new capability to its long-term owner:

- backend-independent frame, layout, paint, inspection, interaction, animation, invalidation, and retained lifecycle contracts belong in `zeterm/zui`;
- reusable UI components belong in `zeterm/ui` (the `zeta-ui` crate);
- file, SCM, editor, terminal, and other domain behavior belongs in its domain crate;
- `zeterm` owns product state mapping, platform event adaptation, renderer composition, and the native product host.

Changes in `zeta-rs/native` are limited to compatibility needed to remove or migrate old implementation, thin mapping from platform/product state into canonical lower APIs, and minimal wiring required to keep the existing host running. An exception must name the long-term owner, migration endpoint, and deletion condition.

When modifying existing native files, prefer migrating, deleting, deprecating, or narrowing responsibility. Historical split scene/interaction hosting is debt, not a foundation for new helpers or public abstractions.
