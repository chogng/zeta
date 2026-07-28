import { markAsDisposed, trackDisposable, toDisposable, } from "./lifecycle.js";
/**
 * A small synchronous event source with disposable listener registrations.
 *
 * Registrations are independent even when they use the same listener
 * function. Reentrant events are delivered after the current event finishes
 * so every listener observes events in FIFO order.
 */
export class Emitter {
    #listeners = new Set();
    #deliveryQueue = [];
    #onListenerError;
    #delivering = false;
    #disposed = false;
    event = (listener) => {
        if (this.#disposed) {
            throw new ReferenceError("Emitter is already disposed");
        }
        const registration = {
            listener,
            active: true,
        };
        this.#listeners.add(registration);
        return toDisposable(() => {
            registration.active = false;
            this.#listeners.delete(registration);
        });
    };
    constructor(options = {}) {
        this.#onListenerError =
            options.onListenerError ?? reportListenerError;
        trackDisposable(this);
    }
    fire(event) {
        if (this.#disposed)
            return;
        for (const registration of this.#listeners) {
            this.#deliveryQueue.push({ registration, event });
        }
        if (this.#delivering)
            return;
        this.#delivering = true;
        try {
            for (let index = 0; index < this.#deliveryQueue.length; index += 1) {
                const delivery = this.#deliveryQueue[index];
                if (!delivery.registration.active)
                    continue;
                try {
                    delivery.registration.listener(delivery.event);
                }
                catch (error) {
                    this.#reportListenerError(error);
                }
            }
        }
        finally {
            this.#deliveryQueue.length = 0;
            this.#delivering = false;
        }
    }
    dispose() {
        if (this.#disposed)
            return;
        this.#disposed = true;
        try {
            for (const registration of this.#listeners) {
                registration.active = false;
            }
            this.#listeners.clear();
            this.#deliveryQueue.length = 0;
        }
        finally {
            markAsDisposed(this);
        }
    }
    [Symbol.dispose]() {
        this.dispose();
    }
    #reportListenerError(error) {
        try {
            this.#onListenerError(error);
        }
        catch (reportingError) {
            reportListenerError(error);
            reportListenerError(reportingError);
        }
    }
}
function reportListenerError(error) {
    if (typeof globalThis.reportError === "function") {
        globalThis.reportError(error);
        return;
    }
    console.error("Unexpected error in event listener", error);
}
