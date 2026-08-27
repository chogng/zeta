---
name: vscode-file-convergence
description: Refactor or retire a project file by comparing its responsibility with the matching VS Code source, then aligning it with the local corresponding file while updating references, tests, and documentation.
---

# VS Code File Convergence

Use this skill when the user asks to refactor, restructure, consolidate, “重构”, “收掉”, “退场”, retire, or remove a file or responsibility in a VS Code-derived area, especially when the result should correspond to the VS Code source layout. If the prompt says “this path”, “under this directory”, or equivalent, include the path supplied by the prompt and all of its descendants; resolve that scope for the current task instead of hardcoding a path from an earlier task.

## Outcome

The local implementation has one canonical owner in the file corresponding to the relevant VS Code source, and all callers, tests, build metadata, and current architecture documentation point to that owner. When the user asked for retirement, the original file is also removed; when the user asked only for refactoring, preserve files that still have a legitimate responsibility.

## Workflow

1. Resolve the target file from the user's path or active editor context. If the target is ambiguous, ask before editing.
2. Read the repository's `AGENTS.md`, the root repository instructions, and every scoped instruction matching both the target and the files that will receive the moved responsibility. Preserve unrelated working-tree changes.
3. Identify the VS Code reference root from repository instructions or nearby workspace layout. In this project it is commonly `../vscode`. Inspect the reference file by responsibility: compare its exports, callers, lifecycle/ownership role, side effects, and neighboring modules. Do not edit the reference VS Code tree unless the user explicitly asks for a cross-repository change.
4. Locate the local counterpart of the reference file and verify the mapping semantically. Prefer an established path mapping such as `src/vs/...` to `zeta-ts/src/zeta/...`, but do not assume a matching basename is sufficient.
5. Merge the target's concrete responsibility into that local counterpart. Preserve local APIs and project-specific behavior unless the comparison shows that the user explicitly wants alignment; keep dependency direction and the counterpart's existing ownership intact. Do not copy unrelated upstream code or create a speculative abstraction.
6. Update imports, exports, tests, generated/build manifests, and documentation that name the old module or describe its old ownership. For retirement, remove the old file only after a repository-wide search shows no required references remain; do not leave a compatibility shim unless an explicit external contract requires one. For refactoring, keep or rename the old file only when it still owns a distinct responsibility.
7. Verify the result with a repository-wide stale-reference search, the smallest relevant typecheck and tests, and a focused diff/status review. Report any pre-existing validation failures separately from failures introduced by the migration.

## Decision boundaries

- Compare responsibilities, not just filenames. If the VS Code reference has no safe local counterpart, or two local files plausibly own the same responsibility, stop and explain the evidence instead of guessing.
- Treat “this path” as a prompt-time scope: use the nearest explicit path in the user's request, or the active file's containing directory when the IDE context clearly supplies it, and include its descendant files. Never turn that resolved example path into a permanent skill-specific path.
- The VS Code tree is a reference for ownership and correspondence, not an authorization to broaden the change or overwrite local behavior.
- Keep the change inside the current project. Follow the project's `AGENTS.md` boundaries, including any restrictions on native paths or naming.
- A retired file may have a dedicated behavior test retained under its feature name; move the test only when doing so improves the canonical ownership boundary and does not reduce coverage.
