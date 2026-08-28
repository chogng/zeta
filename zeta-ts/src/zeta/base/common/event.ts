import { AbstractDisposable, DisposableStore, noneDisposable, toDisposable, type IDisposable } from './lifecycle.js';
import { onUnexpectedError } from './errors.js';

export interface Event<T> {
	(listener: (event: T) => unknown, thisArgs?: unknown, disposables?: IDisposable[] | DisposableStore): IDisposable;
}

export namespace Event {
	export const None: Event<any> = () => noneDisposable;
}

export interface EmitterOptions {
	readonly onWillAddFirstListener?: (emitter: Emitter<any>) => void;
	readonly onDidAddFirstListener?: (emitter: Emitter<any>) => void;
	readonly onDidAddListener?: (emitter: Emitter<any>) => void;
	readonly onWillRemoveListener?: (emitter: Emitter<any>) => void;
	readonly onDidRemoveLastListener?: (emitter: Emitter<any>) => void;
	readonly onListenerError?: (error: unknown) => void;
}

interface ListenerRegistration<T> {
	readonly listener: (event: T) => unknown;
	readonly thisArgs: unknown;
	isActive: boolean;
}

interface EventDelivery<T> {
	readonly registration: ListenerRegistration<T>;
	readonly event: T;
}

interface BufferedEventDelivery {
	deliver(): void;
}

let activeEventBuffer: BufferedEventDelivery[] | undefined;

export function runWithBufferedEvents<T>(mutation: () => T): T {
	if (typeof mutation !== 'function') {
		throw new TypeError('Buffered event mutation must be a function');
	}

	const inherited = activeEventBuffer;
	if (inherited) {
		const savepoint = inherited.length;
		try {
			return mutation();
		} catch (error) {
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
		if (typeof (result as { readonly then?: unknown } | undefined)?.then === 'function') {
			throw new TypeError('Buffered event mutations must be synchronous');
		}
	} catch (error) {
		buffer.length = 0;
		failure = error;
		failed = true;
	} finally {
		activeEventBuffer = undefined;
	}

	if (failed) {
		throw failure;
	}
	for (const delivery of buffer) {
		delivery.deliver();
	}
	return result;
}

export class Emitter<T> extends AbstractDisposable {
	private readonly listeners = new Set<ListenerRegistration<T>>();
	private readonly deliveryQueue: EventDelivery<T>[] = [];
	private isDelivering = false;

	public readonly event: Event<T> = (listener, thisArgs, disposables) => {
		this.assertNotDisposed();
		const isFirstListener = this.listeners.size === 0;
		if (isFirstListener) {
			this.options.onWillAddFirstListener?.(this);
		}

		const registration: ListenerRegistration<T> = { listener, thisArgs, isActive: true };
		this.listeners.add(registration);
		if (isFirstListener) {
			this.options.onDidAddFirstListener?.(this);
		}
		this.options.onDidAddListener?.(this);

		const disposable = toDisposable(() => this.removeListener(registration));
		return addDisposable(disposable, disposables);
	};

	constructor(private readonly options: EmitterOptions = {}) {
		super();
	}

	public fire(event: T): void {
		if (this.isDisposed || this.listeners.size === 0) {
			return;
		}

		const deliveries = [...this.listeners].map(registration => ({ registration, event }));
		if (activeEventBuffer) {
			activeEventBuffer.push({ deliver: () => this.enqueue(deliveries) });
			return;
		}
		this.enqueue(deliveries);
	}

	public hasListeners(): boolean {
		return this.listeners.size > 0;
	}

	protected override disposeCore(): void {
		const hadListeners = this.listeners.size > 0;
		for (const registration of this.listeners) {
			registration.isActive = false;
		}
		this.listeners.clear();
		this.deliveryQueue.length = 0;
		if (hadListeners) {
			this.options.onDidRemoveLastListener?.(this);
		}
	}

	private removeListener(registration: ListenerRegistration<T>): void {
		if (!registration.isActive) {
			return;
		}
		this.options.onWillRemoveListener?.(this);
		registration.isActive = false;
		this.listeners.delete(registration);
		if (this.listeners.size === 0) {
			this.options.onDidRemoveLastListener?.(this);
		}
	}

	private enqueue(deliveries: readonly EventDelivery<T>[]): void {
		this.deliveryQueue.push(...deliveries);
		if (this.isDelivering) {
			return;
		}

		this.isDelivering = true;
		try {
			for (let index = 0; index < this.deliveryQueue.length; index += 1) {
				const delivery = this.deliveryQueue[index];
				if (!delivery.registration.isActive) {
					continue;
				}
				try {
					delivery.registration.listener.call(delivery.registration.thisArgs, delivery.event);
				} catch (error) {
					this.reportListenerError(error);
				}
			}
		} finally {
			this.deliveryQueue.length = 0;
			this.isDelivering = false;
		}
	}

	private reportListenerError(error: unknown): void {
		try {
			(this.options.onListenerError ?? onUnexpectedError)(error);
		} catch (reportingError) {
			console.error('Unexpected error while reporting an event listener error', error, reportingError);
		}
	}
}

function addDisposable<T extends IDisposable>(disposable: T, target: IDisposable[] | DisposableStore | undefined): T {
	try {
		if (Array.isArray(target)) {
			target.push(disposable);
		} else {
			target?.add(disposable);
		}
		return disposable;
	} catch (error) {
		disposable.dispose();
		throw error;
	}
}
