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

- Use targeted `cargo test -p <crate>` or a narrower test target.

## app

- Assert state, commands, semantic identity, events, timing, output, and PTY lifecycle. Do not use screenshots or pixels as pass/fail evidence.
- Validate the running product with `just app`, `python3 -B build/cargo_with_v8.py run -p app`, or the built executable. Use `APP_SESSION_TRACE=1`; add `APP_SESSION_TRACE_FRAMES=1` only for frame timing.
