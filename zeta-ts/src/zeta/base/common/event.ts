import {
	AbstractDisposable,
	type IDisposable,
	noneDisposable,
	toDisposable,
} from "./lifecycle.js";
import { onUnexpectedError } from "./errors.js";

/** A function that subscribes a listener and returns its registration. */
export interface Event<T> {
	(listener: (event: T) => void): IDisposable;
}

/** Event source for boundaries that intentionally never publish. */
export function noEvent<T>(_listener: (event: T) => void): IDisposable {
	return noneDisposable;
}

/** Error reporting policy for one event source. */
export interface EmitterOptions {
	/** Runs immediately before the first listener registration is added. */
	readonly onWillAddFirstListener?: () => void;
	/** Runs after the final listener registration is removed. */
	readonly onDidRemoveLastListener?: () => void;
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

interface BufferedEventDelivery {
	deliver(): void;
}

let activeEventBuffer: BufferedEventDelivery[] | undefined;

/**
 * Defers synchronous event delivery until a related group of state mutations
 * has completed. If the mutation throws, buffered events are discarded.
 */
export function runWithBufferedEvents<T>(mutation: () => T): T {
	if (typeof mutation !== "function") throw new TypeError("Buffered event mutation must be a function");
	const inherited = activeEventBuffer;
	if (inherited) {
		const savepoint = inherited.length;
		try { return mutation(); }
		catch (error) {
			inherited.length = savepoint;
			throw error;
		}
	}
	const buffer: BufferedEventDelivery[] = [];
	activeEventBuffer = buffer;
	let result!: T;
	let failure: unknown;
	let failed = false;
	try {
		result = mutation();
		if (typeof (result as { readonly then?: unknown } | undefined)?.then === "function") throw new TypeError("Buffered event mutations must be synchronous");
	} catch (error) {
		buffer.length = 0;
		failure = error;
		failed = true;
	} finally {
		activeEventBuffer = undefined;
	}
	if (failed) throw failure;
	for (const delivery of buffer) delivery.deliver();
	return result;
}

/**
 * A small synchronous event source with disposable listener registrations.
 *
 * Registrations are independent even when they use the same listener
 * function. Reentrant events are delivered after the current event finishes
 * so every listener observes events in FIFO order.
 */
export class Emitter<T> extends AbstractDisposable {
	private readonly listeners = new Set<ListenerRegistration<T>>();
	private readonly deliveryQueue: EventDelivery<T>[] = [];
	private readonly onListenerError: (error: unknown) => void;
	private readonly onWillAddFirstListener: (() => void) | undefined;
	private readonly onDidRemoveLastListener: (() => void) | undefined;
	private delivering = false;

	readonly event: Event<T> = (listener) => {
		this.assertNotDisposed();
		if (this.listeners.size === 0) {
			this.onWillAddFirstListener?.();
		}
		const registration: ListenerRegistration<T> = {
			listener,
			active: true,
		};
		this.listeners.add(registration);
		return toDisposable(() => {
			if (!registration.active) return;
			registration.active = false;
			this.listeners.delete(registration);
			if (this.listeners.size === 0) {
				this.onDidRemoveLastListener?.();
			}
		});
	};

	constructor(options: EmitterOptions = {}) {
		super();
		this.onWillAddFirstListener = options.onWillAddFirstListener;
		this.onDidRemoveLastListener = options.onDidRemoveLastListener;
		this.onListenerError =
			options.onListenerError ?? onUnexpectedError;
	}

	fire(event: T): void {
		if (this.isDisposed) return;
		const deliveries = [...this.listeners].map(registration => ({ registration, event }));
		if (activeEventBuffer) {
			activeEventBuffer.push({ deliver: () => this.enqueue(deliveries) });
			return;
		}
		this.enqueue(deliveries);
	}

	private enqueue(deliveries: readonly EventDelivery<T>[]): void {
		this.deliveryQueue.push(...deliveries);
		if (this.delivering) return;
		this.delivering = true;
		try {
			for (let index = 0; index < this.deliveryQueue.length; index += 1) {
				const delivery = this.deliveryQueue[index];
				if (!delivery.registration.active) continue;
				try {
					delivery.registration.listener(delivery.event);
				} catch (error) {
					this.reportListenerError(error);
				}
			}
		} finally {
			this.deliveryQueue.length = 0;
			this.delivering = false;
		}
	}

	protected override disposeCore(): void {
		const hadListeners = this.listeners.size > 0;
		for (const registration of this.listeners) {
			registration.active = false;
		}
		this.listeners.clear();
		this.deliveryQueue.length = 0;
		if (hadListeners) this.onDidRemoveLastListener?.();
	}

	private reportListenerError(error: unknown): void {
		try {
			this.onListenerError(error);
		} catch (reportingError) {
			console.error("Unexpected error while reporting an event listener error", error, reportingError);
		}
	}
}
