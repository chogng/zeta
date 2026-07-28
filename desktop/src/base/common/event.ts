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

/** Error reporting policy for one event source. */
export interface EmitterOptions {
  /**
   * Receives errors thrown by listeners after delivery continues to the other
   * registrations.
   */
  readonly onListenerError?: (error: unknown) => void;
}

interface ListenerRegistration<T> {
  readonly listener: (event: T) => void;
  active: boolean;
}

interface EventDelivery<T> {
  readonly registration: ListenerRegistration<T>;
  readonly event: T;
}

/**
 * A small synchronous event source with disposable listener registrations.
 *
 * Registrations are independent even when they use the same listener
 * function. Reentrant events are delivered after the current event finishes
 * so every listener observes events in FIFO order.
 */
export class Emitter<T> implements IDisposable {
  readonly #listeners = new Set<ListenerRegistration<T>>();
  readonly #deliveryQueue: EventDelivery<T>[] = [];
  readonly #onListenerError: (error: unknown) => void;
  #delivering = false;
  #disposed = false;

  readonly event: Event<T> = (listener) => {
    if (this.#disposed) {
      throw new ReferenceError("Emitter is already disposed");
    }
    const registration: ListenerRegistration<T> = {
      listener,
      active: true,
    };
    this.#listeners.add(registration);
    return toDisposable(() => {
      registration.active = false;
      this.#listeners.delete(registration);
    });
  };

  constructor(options: EmitterOptions = {}) {
    this.#onListenerError =
      options.onListenerError ?? reportListenerError;
    trackDisposable(this);
  }

  fire(event: T): void {
    if (this.#disposed) return;
    for (const registration of this.#listeners) {
      this.#deliveryQueue.push({ registration, event });
    }
    if (this.#delivering) return;
    this.#delivering = true;
    try {
      for (let index = 0; index < this.#deliveryQueue.length; index += 1) {
        const delivery = this.#deliveryQueue[index];
        if (!delivery.registration.active) continue;
        try {
          delivery.registration.listener(delivery.event);
        } catch (error) {
          this.#reportListenerError(error);
        }
      }
    } finally {
      this.#deliveryQueue.length = 0;
      this.#delivering = false;
    }
  }

  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    try {
      for (const registration of this.#listeners) {
        registration.active = false;
      }
      this.#listeners.clear();
      this.#deliveryQueue.length = 0;
    } finally {
      markAsDisposed(this);
    }
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  #reportListenerError(error: unknown): void {
    try {
      this.#onListenerError(error);
    } catch (reportingError) {
      reportListenerError(error);
      reportListenerError(reportingError);
    }
  }
}

function reportListenerError(error: unknown): void {
  if (typeof globalThis.reportError === "function") {
    globalThis.reportError(error);
    return;
  }
  console.error("Unexpected error in event listener", error);
}
