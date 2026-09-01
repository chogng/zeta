---
name: document-placement
description: Choose the owning project's docs directory when creating or relocating development and design documents in Zeta. Use for architecture documents, technical designs, implementation plans, and engineering proposals; do not use it to relocate existing documents or README files unless the user explicitly asks.
---

# Document Placement

Before creating a development or design document, identify the project that owns the documented capability and place the document in that project's `docs/` directory.

| Document owner | Destination |
| --- | --- |
| TypeScript frontend under `zeta-ts/`, including the Zeta editor and Workbench | `zeta-ts/docs/` |
| Shared Rust backend under `zeta-rs/` | `zeta-rs/docs/` |
| Desktop product and Rust UI under `app/` | `app/docs/` |
| CLI and TUI product under `zeta-code/` | `zeta-code/docs/` |
| Repository-wide behavior that genuinely has no single project owner | `docs/` |

Apply the same rule to another stable top-level project: use `<project>/docs/`. Do not treat an individual crate, package, feature folder, or the currently active file as a project boundary.

Choose the owner from the document's subject and long-term responsibility. When a document crosses projects but one project owns the contract or user-visible behavior, place it with that owner and link to supporting implementation elsewhere. Split the document only when the topics have independent owners and can remain useful independently.

Keep existing documents at their current paths when editing them. Do not migrate, rename, or reorganize existing documentation merely to adopt this rule. A relocation explicitly requested by the user may use the destination rules above.

Do not move, rename, or convert any `README.md` under this skill. README files remain beside the implementation they describe. Do not create or update documentation indexes unless the task requests it.

Follow the repository's Markdown and documentation instructions after choosing the destination. If the user explicitly supplies a destination for the current document, follow that request.
