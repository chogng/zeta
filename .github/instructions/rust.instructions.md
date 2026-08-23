---
description: Zeta Rust API, module, import, documentation, ownership, and file-size conventions.
applyTo: "**/*.rs"
---

# Rust Coding Guidelines

- Newly added traits include doc comments explaining their role and how implementations use them.
- Avoid boolean and ambiguous `Option` parameters. Prefer enums, named methods, or newtypes.
- Default modules and implementation details to private and explicitly re-export the public crate API.
- Prefer one Rust import per line over brace-grouped imports.
- Use file-based module roots: `foo.rs` and `foo/bar.rs`. Do not introduce `foo/mod.rs` without an external constraint.
- Prefer production modules below 500 lines. Near 800 lines, add new responsibility in another module and move its tests and documentation with it.

For new test modules, use a separate sibling file and an explicit descriptive path:

```rust
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

- Backend-neutral protocols, domains, storage, execution, terminal semantics, and server hosting belong in `zeta-rs`; product presentation and host composition do not.
