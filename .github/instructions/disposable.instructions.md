---
description: Guidelines for writing code using IDisposable
---

Core symbols:

* `IDisposable` implements both `dispose()` and `[Symbol.dispose]()`; disposal is synchronous and idempotent.
* `Disposable` is the VS Code-style base class for composite objects. It exposes the protected `_store` and `_register<T>()`, and provides `Disposable.None` for an intentionally empty resource.
* `AbstractDisposable` is for stateful leaf resources with custom cleanup in `disposeCore()`; composite objects should extend `Disposable` instead.
* `DisposableStore` owns resources in LIFO order. `clear()` disposes the current contents and leaves the store reusable; `isDisposed` reports the terminal state.
* `MutableDisposable<T>` owns one replaceable resource through its `value` setter. Replacing or clearing disposes the previous value; `clearAndLeak()` transfers the current value without disposing it.
* `AsyncDisposableStore` owns synchronous and asynchronous resources and releases them asynchronously in LIFO order.
* `DisposableMap` owns resources whose lifetime follows stable keys.
* `toDisposable(fn)` adapts one cleanup callback to `IDisposable`.

Register resources immediately after creating them:

```ts
class TitlebarPart extends Disposable {
	private readonly listener = this._register(titleService.onDidChange(() => this.render()));
}
```

Use `toDisposable()` when the cleanup is a callback or a value has no project disposable wrapper:

```ts
this._register(toDisposable(() => element.remove()));
```

Use `using` for short lexical scopes. Both explicit disposal and `[Symbol.dispose]()` are supported:

```ts
using resource = service.subscribe(listener);
```

Registration after disposal throws `ReferenceError` and does not take ownership of the rejected resource. Tracker hooks record ownership, reject multiple owners and cycles, and must remain optional diagnostics rather than lifecycle prerequisites.
