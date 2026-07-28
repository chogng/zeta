---
description: Zeta TypeScript resource ownership and cancellation guidelines
---

# Lifecycle

Zeta uses the ECMAScript explicit resource-management protocol as its
foundation and exposes a small project facade for application code.

## Protocols

- `Disposable`, `[Symbol.dispose]()` and `using` are the synchronous protocol.
- `AsyncDisposable`, `[Symbol.asyncDispose]()` and `await using` are the
  asynchronous protocol.
- `AbortSignal` represents cancellation of one asynchronous operation. It does
  not represent ownership and must not dispose the object performing the work.
- Keep synchronous and asynchronous cleanup separate. Do not define
  `dispose(): void | Promise<void>`.

Lifecycle and cancellation are separate base modules:

```text
base/common/
├─ lifecycle.ts
└─ cancellation.ts
```

`lifecycle.ts` must not depend on `cancellation.ts`. The cancellation module
uses the standard `AbortSignal` protocol and owns only project-wide
cancellation policy such as `CancellationError`, classification, signal
composition, and timeout helpers. Add the latter helpers only when real
callers need them.

## Project facade

- `IDisposable extends Disposable` adds the convenient explicit `dispose()`
  entry point. Project-created synchronous resources should implement both
  entry points idempotently.
- `IAsyncDisposable extends AsyncDisposable` adds `disposeAsync()`.
- `DisposableStore` owns a fixed-lifetime group and releases it in LIFO order.
- `AsyncDisposableStore` owns asynchronous and synchronous resources and
  releases them asynchronously in LIFO order.
- `DisposableOwner` is an optional base class for long-lived objects. Its `own`,
  `adopt`, and `defer` helpers transfer resources into the object's store.
- `DisposableSlot<T>` owns one replaceable resource.
- `ResettableDisposableGroup` is only for a real clear-and-rebuild lifecycle.
- `toDisposable()` adapts one cleanup callback to `IDisposable`.

Public ownership inputs should accept the standard `Disposable` interface.
Project APIs that create a resource should return `IDisposable` so callers may
use either `.dispose()` or `using`.

## Ownership rules

Register a resource immediately after creating it:

```ts
class TitlebarPart extends DisposableOwner {
  readonly #listener = this.own(
    titleService.onDidChange(() => this.render()),
  );
}
```

Use `using` for lexical, short-lived ownership:

```ts
function performOperation(): void {
  using subscription = service.subscribe(listener);
  using lock = acquireLock();
  doWork();
}
```

Use composition when a class already has a natural base class:

```ts
class Component extends ExistingBase implements IDisposable {
  readonly #resources = new DisposableStore();

  dispose(): void {
    this.#resources.dispose();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }
}
```

Disposal must be idempotent. A disposed store or slot rejects new resources
with `ReferenceError`; it does not silently dispose or adopt the incoming
resource. Callers therefore retain ownership when registration fails.

## Electron boundary

Electron `contextBridge` cannot carry Symbol-keyed properties. A preload API
therefore returns a string-keyed `DisposableHandle` containing only
`dispose()`. This is a serialization-boundary type, not the local lifecycle
protocol. Adapt it to `IDisposable` with `toDisposable()` when renderer code
needs to transfer the handle into a local owner.

Do not create general-purpose adapters elsewhere. Standard `Disposable`
structural typing already provides local interoperability.

## Diagnostics

`DisposableTracker` is an opt-in development and test diagnostic. It records
creation stacks and owner-child relationships, rejects multiple owners and
ownership cycles, and can assert that a completed scope has no live resources:

```ts
const tracker = new DisposableTracker();
using tracking = installDisposableTracker(tracker);

const store = new DisposableStore();
store.add(service.subscribe(listener));
store.dispose();

tracker.assertNoLeaks();
```

Only one tracker is installed per JavaScript realm. Zeta's Electron main and
renderer entry points install it automatically in development and assert their
respective application scopes during shutdown.

Tracking is disabled when no tracker is installed, so production lifecycle
correctness must never depend on it.

The thin `IDisposableTracker` contract, current tracker slot, and notification
hooks live in `lifecycle.ts`. Stack collection, ownership graphs, and leak
reporting live in `disposableTracker.ts`; the lifecycle module does not depend
on that concrete implementation.

## Review rules

- Register a newly created resource immediately with `own()`, `add()`, a slot,
  or `using`.
- Do not discard a returned `IDisposable` unless the API explicitly documents
  process-lifetime ownership.
- Do not register one resource with multiple owners.
- Do not override `DisposableOwner.dispose()` or `[Symbol.dispose]()`.
- Cleanup callbacks must tolerate repeated public disposal, even though the
  standard stacks already guard their own state.
- A synchronous owner must not hide unfinished asynchronous cleanup.
- Keep `AbortSignal` cancellation separate from resource disposal.
- Add a semantic test whenever a new resource container or ownership pattern is
  introduced.
