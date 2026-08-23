# TypeScript

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


# Code formatting

- Prefer compact single-line formatting for short imports, function calls, parameter lists, conditions, and expressions.
- Split code across lines only when it exceeds the project's line-length convention, materially improves readability, or is required by the configured formatter.
- Do not preemptively use multiline formatting merely because more items might be added later.

## Learnings

- 严禁创建或保留仅用于聚合、转发导出的 `index.ts`（barrel file）。所有调用方必须直接从符号的实际定义模块导入，使依赖来源、代码 owner 和运行时边界保持明确；发现现有 barrel 时应删除并改写调用方，不得继续扩展。
