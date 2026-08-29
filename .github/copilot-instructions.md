# Zeta Repository Instructions

## Quick reference

Before changing a file, identify its owner and read every matching scoped instruction.

| Change surface | Required instruction |
| --- | --- |
| TypeScript frontend | [`coding-guidelines.instructions.md`](instructions/coding-guidelines.instructions.md) and [`frontend-architecture.instructions.md`](instructions/frontend-architecture.instructions.md) |
| Editor implementation | [`editor.instructions.md`](instructions/editor.instructions.md) |
| Browser UI or CSS | [`browser-ui.instructions.md`](instructions/browser-ui.instructions.md) |
| Rust | [`rust.instructions.md`](instructions/rust.instructions.md) |
| `zeta-rs/native` or `app` | [`native.instructions.md`](instructions/native.instructions.md) |
| Tests | [`testing.instructions.md`](instructions/testing.instructions.md) |
| Markdown documentation | [`documentation.instructions.md`](instructions/documentation.instructions.md) |
| `zeta-code` CLI/TUI | [`tui.instructions.md`](instructions/tui.instructions.md) |

Scoped instructions contain implementation rules. Architecture documents contain design, status, and rationale.

### Finding Related Code
1. **Semantic search first**: Use file search for general concepts
2. **Grep for exact strings**: Use grep for error messages or specific function names
3. **Follow imports**: Check what files import the problematic module
4. **Check test files**: Often reveal usage patterns and expected behavior


## Repository ownership

Desktop frontend paths below are relative to `zeta-ts/`.

| Path | Owner |
| --- | --- |
| `src/zeta/base` | Domain-neutral TypeScript utilities and UI primitives |
| `src/zeta/platform` | Shared frontend services and platform abstractions |
| `src/zeta/editor` | Editor models, state, projection, and contributions |
| `src/zeta/workbench` | Application shell, Parts, panes, and product composition |
| `zeta-rs` | Shared Rust backend protocols, domains, storage, execution, terminal semantics, and backend-neutral server host |
| `app` | Rust Desktop product, including `zui`, `zeta-ui-components`, `zeta-workbench-ui`, renderer, `wgpu`, and `winit` |
| `zeta-code` | `zeta code` CLI and Ratatui product host |

Preserve the frontend dependency direction `base → platform → editor → workbench`. Lower layers must not import, specialize for, or copy state from higher layers. Multiple callers do not justify moving a domain concept into `base`; the abstraction must be domain-neutral and have a complete current consumer contract.

`zeta-ts` and `app` must not execute, package, import, or depend on `zeta-cli`, `zeta-tui`, `zeta-code/cli`, or the `zeta app-server` product command. Shared backend process entrypoints belong to `zeta-rs/server-host`.

`zeta-rs`, `app`, and `zeta-code` may remain in the same root Cargo workspace; workspace membership does not change implementation ownership.

When a request mentions Workbench, Sessions, or another frontend concept, locate it in the Renderer/Workbench first. Only route to `zeta-code` when the request explicitly concerns the terminal, Ratatui, the CLI, or `zeta-code`.

## General implementation rules

- Give state, lifecycle, layout, rendering, persistence, and validation one canonical owner.
- Keep adapters thin and mechanical. Product consumers depend on frontend or backend domain contracts, not generated transport DTOs.
- Reuse an existing abstraction when its contract is complete for the caller. Do not create speculative abstractions, empty placeholder modules, or public APIs without a concrete consumer.
- Register owned disposable resources immediately after creation. A repeated resource must be disposed with the repeated state that owns it.
- Prefer direct dependencies and method calls over hidden service lookup or event chains used as control flow.
- Keep public APIs smaller than implementations. Export only symbols required by another module or a documented external contract.
- Validate at user, configuration, plugin, persistence, process, worker, IPC, RPC, and network boundaries. Trust typed same-process values after the owning boundary has normalized them.

## Cross-system ownership

Workspace owns source scanning, ignore rules, reads, chunking, revision, and chunk identity. A cloud CodeIndex may prepare model input, perform vector retrieval, call rerank, and sort/filter/truncate by model score, but it may consume only exact Workspace-authorized chunks. `model-provider` owns model invocation, not indexing or retrieval policy; cross-source result fusion belongs to retrieval.

Callable Skills use the dedicated `$name` selector, such as `$commit`. Slash Commands remain product or server commands, while `@` remains the selector prefix for files and Plugin-provided context. `/skills` owns browsing, enablement, and diagnostics. Skill lists load metadata only; full `SKILL.md` content loads after selection or automatic activation.

Generic Marketplace infrastructure and templates must not hardcode a repository owner, personal namespace, or product identity. Publisher identity comes from repository configuration; product-specific manifests belong only to optional consumer adapters. Marketplace validation and publication must not require any consuming product source tree or release state.

## Development workflow

- Modify the exact repository or worktree the user named. Do not substitute a temporary clone, another worktree, or only a remote branch without explicit agreement.
- In an owner-led pre-release repository, a PR is not the default integration requirement. Use PR review for external contributions, security-sensitive work, or real multi-person review needs.
- Use the smallest typecheck, test, build, or documentation check that covers the changed surface. Rust package tests use `just test <crate> [args]`; see [`testing.instructions.md`](instructions/testing.instructions.md) for the package-selection rule. Do not report a command as passing unless it completed successfully.
- Preserve unrelated working-tree changes. A historical violation is migration debt, not precedent for new code.
