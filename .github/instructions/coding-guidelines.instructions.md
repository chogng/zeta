---
description: Zeta TypeScript frontend naming, formatting, API, and code quality rules.
applyTo: "**/src/**/*.ts,**/src/**/*.cts,**/test/**/*.ts"
---

# TypeScript Coding Guidelines

Applies to the entire frontend. Domain instructions may add rules, but must not redefine them.

## Formatting

- Use tabs, single quotes, and semicolons.
- Keep each import declaration on one physical line.
- Keep short calls, signatures, conditions, and expressions on one line.
- Use about 120 columns as the normal working width. Imports may exceed it.
- When wrapping, use one logical item per line and a trailing comma. Do not wrap for hypothetical growth.
- Always use braces; put the opening brace on the same line.
- Use blank lines only between real reading phases.

Existing files are not yet migrated. Preserve their indentation, quotes, and private-member style; do not reformat unrelated lines. New files use these rules.

## Naming

- PascalCase for classes, interfaces, types, enums, and enum members.
- camelCase for functions, methods, properties, parameters, and local variables.
- Use whole, precise domain words. Avoid generic `Context`, `Manager`, `Helper`, and `Data` names.
- Boolean names normally start with `is`, `has`, `can`, or `should`.
- Event handlers use `handle*`. Events use `onDid*`.

## Class Members

- Declare visibility explicitly.
- Do not prefix private or protected members with `_`. Use `_` only for a backing member that shares a public semantic name.
- Default to `private`. Use `protected` only for an intentional subclass API.
- Constructors omit redundant `public`; use `private` or `protected` only to restrict construction.
- Use `readonly` when the field reference is assigned once.
- Access instance-owned state through `this`; keep inputs and intermediate values local.

```ts
public readonly domNode: HTMLElement;
private readonly model: TextModel;
private visible = false;
```

## Disposable Ownership

- Treat `IDisposable` as a cleanup capability and `DisposableOwner` as a composite ownership mechanism. Choose from the object's ownership role, not to avoid writing `dispose()`.
- A leaf adapter or handle whose identity is the cleanup operation implements `IDisposable` directly. Its cleanup is synchronous and idempotent, supports `[Symbol.dispose]()`, and participates in disposable tracking.
- A component that aggregates independently created listeners, child components, timers, or replaceable resources normally extends `DisposableOwner` and registers each resource immediately with `own`, `adopt`, or `defer`.
- Owning one implementation resource does not by itself make a leaf adapter a composite owner. Do not allocate a `DisposableStore` or extend `DisposableOwner` only to delegate disposal to that resource.
- When inheritance is unavailable, use a component-owned `DisposableStore`; do not hand-maintain an array of cleanup callbacks.

## Functions

- Named functions and class methods declare return types.
- Prefer guard clauses. Omit `else` after a terminal branch.
- Prefer `async`/`await` over `.then()` chains.
- Prefer function declarations for exported top-level functions.
- Avoid nested ternaries and condition spreads.
- Constructors establish valid state; they do not perform async work, full rendering, service lookup, or overridable calls.
- Use a private method when it has multiple callers or owns a cohesive state transition; do not use it only to label consecutive statements.

## Types and APIs

- Source is private by default. Use named exports only when another module or contract needs the symbol.
- Use `import type` when an entire import is type-only.
- Add a type only to remove ambiguity, exclude invalid states, or define shared semantics.
- Use an interface for a stable structural contract and a class when the abstraction owns behavior or state.
- Keep one-off constructor options private and only when named fields make the call clearer.
- Use an options object when positional arguments are unclear. Prefer enums, named methods, or options fields over boolean and ambiguous `undefined` arguments.

## Comments

- Default to no comment.
- Prefer clear names, types, and control flow.
- Keep JSDoc to one or two short sentences. Do not restate the signature.
- Inline comments are for non-obvious ordering, lifecycle, compatibility, platform workarounds, units, or measured performance constraints. Do not narrate code or preserve history.

## Code Quality

- Use `const` by default, `let` only when reassigned, and never `var`.
- Do not create speculative abstractions, placeholder modules, or public APIs without a concrete caller.
- Keep short, complete algorithms together. Do not extract helpers that only rename consecutive steps.
- Extract shared semantics, independent lifecycle, complex mechanism, or substantial duplication.
- Judge complexity by concepts and cross-file jumps, not line count alone.
- Prefer production modules below roughly 500 lines. Near 800 lines, add new responsibility in another module unless a documented constraint requires otherwise.
