---
description: Zeta targeted testing and command-line verification rules.
applyTo: "**/test/**,**/*_tests.rs,**/tests/**"
---

# Testing Guidelines

Use the smallest command that covers the changed behavior. Report a pass only after it completes successfully.

## TypeScript frontend

- Name tests in behavior language and keep arrange, act, and assert easy to identify.
- Prefer comparing a complete result over many disconnected field assertions when the full value is the behavior.
- Do not export production helpers solely for tests.
- Prefer state, events, DOM semantics, accessibility, and geometry over screenshots.

## Rust

- Do not run `cargo check` or `cargo test` directly for routine validation. Use `just check <crate> [args]` or `just test <crate> [args]`; these commands select a package and configure the locked V8 files only when its dependency graph needs them.
- Start with the package that owns the changed behavior. A test-name filter does not limit workspace compilation, so always select a package and never run bare `cargo test <filter>` from the workspace root.
- Ask the user before running a complete workspace check or test suite. Escalate only after targeted validation passes and the change affects a shared workspace contract, or when the user explicitly requests full coverage.
- Do not add `--workspace`, `--all-targets`, or `--all-features` as routine validation expansion. Use package-scoped target or feature expansion only when the changed surface specifically requires it; combining expansion with a workspace-wide run requires the same explicit approval as a complete suite.

## app

- Assert state, commands, semantic identity, events, timing, output, and PTY lifecycle. Do not use screenshots or pixels as pass/fail evidence.
- Validate the running product with `just app`, `python3 -B build/cargo_with_v8.py run -p app`, or the built executable. Use `APP_SESSION_TRACE=1`; add `APP_SESSION_TRACE_FRAMES=1` only for frame timing.

## Learnings

* Keep one incremental setting throughout a Rust validation round, and after a failure rerun only the failed test or target; switching artifact modes or rebuilding whole packages creates avoidable duplicate outputs and disk pressure.
