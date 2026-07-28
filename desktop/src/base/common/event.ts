import {
  type IDisposable,
  markAsDisposed,
  trackDisposable,
  toDisposable,
} from "./lifecycle.js";

/** A function that subscribes a listener and returns its registration. */
export interface Event<T> {
  (listener: (event: T) => void): IDisposable;
}

/** A small synchronous event source with disposable listener registrations. */
export class Emitter<T> implements IDisposable {
  readonly #listeners = new Set<(event: T) => void>();
  #disposed = false;

  readonly event: Event<T> = (listener) => {
    if (this.#disposed) {
      throw new ReferenceError("Emitter is already disposed");
    }
    this.#listeners.add(listener);
    return toDisposable(() => {
      this.#listeners.delete(listener);
    });
  };

  constructor() {
    trackDisposable(this);
  }

  fire(event: T): void {
    if (this.#disposed) return;
    for (const listener of [...this.#listeners]) {
      listener(event);
    }
  }

  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    try {
      this.#listeners.clear();
    } finally {
      markAsDisposed(this);
    }
  }

  [Symbol.dispose](): void {
    this.dispose();
  }
}
