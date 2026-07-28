# Communication

- When comparing responsibilities, capabilities, implementation status, or design options, prefer
  a compact, conclusion-oriented comparison table when it makes the distinction clearer.
- Use `✅` and `❌` for genuinely binary judgments so ownership and support boundaries are visually
  unambiguous.
- Do not force nuanced states into a binary marker. Use explicit labels such as `部分具备`,
  `尚未完成`, `协调`, or `委托` when those are more accurate.
- Lead with the conclusion; use surrounding prose only to explain important boundaries or caveats.

# Crates

- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- Prefer private modules and explicitly exported public crate API.
- Avoid large modules:
  - Prefer adding new modules instead of growing existing ones.
  - Target Rust modules under 500 LoC, excluding tests.
  - If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending the existing file unless there is a strong documented reason not to.
  - When extracting code from a large module, move the related tests and module/type docs toward the new implementation so the invariants stay close to the code that owns them.

## Documentation

- Follow [`docs/documentation-guidelines.md`](docs/documentation-guidelines.md).
- Crate READMEs should focus on implementation ownership, exact contracts, execution paths,
  failure semantics, integration obligations, tests, modification impact, current limitations, and
  crate-level extension points.
- Crate READMEs should name the key private symbols that carry ownership, validation, binding,
  failure semantics, and extension direction. Include their real call relationships and identify
  internal changes that would signal architectural drift.
- `docs/*.md` should focus on cross-crate architecture, product semantics, ownership, tradeoffs,
  trust and durability boundaries, current system status, and staged evolution.
- Keep current implementation, proposed work, and potential future directions explicitly
  separated. Do not describe future capability as current behavior.
- When a crate README and a system document cover the same topic, make their canonical ownership
  explicit, link them in both directions, and avoid duplicating the same detailed explanation.

## Tests

### Test module organization

- When adding a new test module, define its contents in a separate sibling file rather than inline in the implementation file.
- Use an explicit `#[path = "..._tests.rs"]` attribute so the test filename is descriptive and easy to locate:

  ```rust
  #[cfg(test)]
  #[path = "parser_tests.rs"]
  mod tests;
  ```
