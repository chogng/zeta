---
description: Zeta Rust package testing, warning, conditional-compilation, protocol-generation, and product validation rules.
applyTo: "**/*.rs,**/Cargo.toml,Cargo.toml,Cargo.lock,justfile,scripts/cargo.py"
---

# Rust Testing Guidelines

Read `testing.instructions.md` first. This file adds Rust-specific commands and coverage requirements.

## Package validation

- Do not run `cargo check` or `cargo test` directly for routine validation. Use `just check <crate> [args]` and `just test <crate> [args]`; these commands select a package and configure the locked V8 files only when its dependency graph needs them.
- Start with the package that owns the changed behavior. A test-name filter does not limit workspace compilation, so always select a package and never run bare `cargo test <filter>` from the workspace root.
- After the package check and affected tests pass, run `just rust-warnings <crate>`; it compiles every package target and denies compiler warnings. Fix the cause instead of adding an `allow` solely to pass the gate.
- Ask the user before running a complete workspace check or test suite. Escalate only after targeted validation passes and the change affects a shared workspace contract, or when the user explicitly requests full coverage.
- Do not add `--workspace`, `--all-targets`, or `--all-features` as routine validation expansion. Use package-scoped target or feature expansion only when the changed surface specifically requires it; combining expansion with a workspace-wide run requires the same explicit approval as a complete suite.
- Keep one incremental setting throughout a validation round. After a failure rerun only the failed test or target; switching artifact modes or rebuilding whole packages creates avoidable duplicate outputs and disk pressure.

## Tests and conditional compilation

- Test-only code must live in a sibling test file whenever the implementation can expose the required private surface to its own test module. Do not add production methods solely to make tests convenient.
- A test-only helper's `cfg` must match every condition shared by all of its callers. For example, a helper used only by a `#![cfg(unix)]` test module uses `#[cfg(all(test, unix))]`, not `#[cfg(test)]`.
- When platform conditions change, compile the owning package's test targets on every affected platform. A successful check on one operating system does not prove another operating system's `cfg` graph.

## Generated contracts

- After changing an App Server protocol type or registry entry, run `just generate-protocol`, the protocol package tests, the generated TypeScript strict check, and the affected client build or typecheck.
- Generated fixtures and bindings are outputs, not independent sources. Update their owner and regenerate them instead of editing generated files directly.

## Rust app validation

- Assert state, commands, semantic identity, events, timing, output, and PTY lifecycle. Do not use screenshots or pixels as pass/fail evidence.
- Validate the running product with `just app`, `python -B scripts/cargo.py run -p app`, or the built executable. Use `APP_SESSION_TRACE=1`; add `APP_SESSION_TRACE_FRAMES=1` only for frame timing.

## Learnings

* Existing test-only helpers must use the same effective `cfg` as all callers; a broader `#[cfg(test)]` can compile dead code on platforms where the caller module is absent. Run the owning package's warning gate after test changes so platform-specific stale helpers fail validation.
