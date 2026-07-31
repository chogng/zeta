# Communication

- When comparing responsibilities, capabilities, implementation status, or design options, prefer
  a compact, conclusion-oriented comparison table when it makes the distinction clearer.
- Use `✅` and `❌` for genuinely binary judgments so ownership and support boundaries are visually
  unambiguous.
- Do not force nuanced states into a binary marker. Use explicit labels such as `部分具备`,
  `尚未完成`, `协调`, or `委托` when those are more accurate.
- Lead with the conclusion; use surrounding prose only to explain important boundaries or caveats.

# Only for Rust Crates

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


# Only for TypeScript

## Interface design

- Design from the caller's point of view: common usage should be concise,
  natural to read, and independent of implementation details.
- Keep interfaces small, complete, and canonical. Every method, option,
  overload, alias, and abstraction must add distinct semantic value.
- Use domain language and the type system to make intent clear and invalid or
  ambiguous calls difficult to express.
- Prefer standard protocols when they already produce clear, interoperable call
  sites.
- Validate interfaces with representative usage. Prefer clear code over clever
  compression or explanatory comments.

## Service boundaries and naming

- Treat service alignment as a repository-wide architecture rule, not a feature-specific convention.
- Put a frontend domain service contract in a `common/*Service.ts` file, and name its public interface
  and service identifier `I<Capability>Service`.
- Name each runtime implementation file after its exported implementation class, including the runtime
  qualifier when it matters, such as `appServerSyntaxAnalysisService.ts` exporting
  `AppServerSyntaxAnalysisService`.
- Keep transport APIs and generated DTOs inside runtime implementation modules. Product consumers must
  depend on the frontend service contract and frontend-owned domain types.
- Align capability names, operation semantics, lifecycle, and error categories across the frontend
  service, transport protocol, and backend service so adapters stay thin and mechanical.
- Name adapter and test files after the contract or implementation they exercise. Do not preserve an
  obsolete transport, editor, or host name after ownership has moved.

## Base module boundaries

- Reverse dependencies from `src/zeta/base` into any higher-level domain are
  strictly prohibited. Features such as PDF, editors, workspaces, sessions, or
  file explorers may depend on base APIs; base modules must never import,
  reference, specialize for, or otherwise depend on those features.
- Keep modules under `src/zeta/base` domain-agnostic. Higher-level domains must not
  determine base interfaces, types, defaults, comparison rules, lifecycle
  behavior, tests, or examples.
- Define URI parsing, URI identity, resource collections, UUID validation, and
  lifecycle primitives in terms of their general contracts rather than a
  current feature's needs or examples.
- Do not make a general resource comparison rule silently ignore URI
  components for one consumer. Preserve exact URI identity by default and let a
  domain explicitly select alternate semantics, such as ignoring fragments.
- Keep domain identities and lifecycle rules, including document IDs and editor
  instance IDs, in the module that owns those concepts. Do not introduce them
  into `src/zeta/base` before a concrete domain model requires them.
- Add structures such as a resource tree when a real hierarchical consumer
  exists. Do not expand the base layer speculatively from anticipated feature
  requirements.

## Renderer UI styling ownership

- Follow [`docs/ui-styling-ownership.md`](docs/ui-styling-ownership.md) for every
  Renderer component, Workbench Part, contribution, theme, and CSS change.
- Before adding or changing a visual rule, identify the owner from the state
  definition, DOM creator, and hosting boundary. The owner must keep its internal
  geometry and interaction-state styles with the component that defines them.
- Follow the VS Code state-projection convention: DOM state must expose a stable
  class such as `.checked` alongside the corresponding ARIA attribute. CSS must
  select the state class rather than using ARIA attributes as visual selectors.
- Workbench Part CSS may own region layout, borders, backgrounds, and the external
  box of a directly hosted component. It must not reach through shared component
  internals to override item, hover, active, focus, selected, or disabled styles.
- Express legitimate visual differences through a named presentation variant or
  semantic token owned by the component. Do not introduce host-specific deep
  selectors or ambiguous boolean styling options.
- Treat historical selectors that violate the canonical document as migration
  debt, not precedent. When modifying an affected area, move the rule to its owner
  or add the required public presentation contract.

## Code formatting

- Every TypeScript import declaration must occupy exactly one physical line. Never wrap imported names or any other part of an import declaration across multiple lines.
- Prefer compact single-line formatting for other short function calls, parameter lists, conditions, and expressions.
- Do not preemptively use multiline formatting merely because more items might be added later.
- Use TypeScript `private` or `protected` members instead of ECMAScript `#private` identifiers so internal call sites read as `this.member`.
- Prefix a private backing member with `_` only when it must coexist with a public member of the same semantic name, such as `_onDidChange` backing `onDidChange`.
