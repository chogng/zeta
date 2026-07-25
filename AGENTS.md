# Crates

- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- Prefer private modules and explicitly exported public crate API.
- Prefer foo.rs over foo/mod.rs for new Rust modules.
- In `lib.rs`, use explicit one-item-per-line:

  ```rust
  mod foo;
  mod bar;
  use foo::InternalFoo;
  pub use bar::Bar;
  ```
- Crates with `src/lib.rs` must declare `[lib]` with `doctest = false`, an underscore-separated `name`, and `path = "src/lib.rs"`; omit it for binary-only crates.
- Define shared dependency versions and local crate paths in `[workspace.dependencies]`; member crates inherit them with `{ workspace = true }`.


## Tests

### Test module organization

- When adding a new test module, define its contents in a separate sibling file rather than inline in the implementation file.
- Use an explicit `#[path = "..._tests.rs"]` attribute so the test filename is descriptive and easy to locate:

  ```rust
  #[cfg(test)]
  #[path = "parser_tests.rs"]
  mod tests;
  ```
