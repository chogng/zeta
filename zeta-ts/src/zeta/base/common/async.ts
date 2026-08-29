import { canceled } from './errors.js';
import { AbstractDisposable } from './lifecycle.js';

export interface CancelablePromise<T> extends Promise<T> {
	cancel(): void;
}

/** Returns a cancelable promise that resolves after the provided timeout. */
export function timeout(millis: number): CancelablePromise<void> {
	let settled = false;
	let rejectPromise!: (reason: unknown) => void;
	let promiseHandle: ReturnType<typeof setTimeout>;
	const promise = new Promise<void>((resolve, reject) => {
		rejectPromise = reject;
		const handle = setTimeout(() => {
			settled = true;
			resolve();
		}, millis);
		promiseHandle = handle;
	}) as CancelablePromise<void>;
	promise.cancel = (): void => {
		if (settled) return;
		settled = true;
		clearTimeout(promiseHandle);
		rejectPromise(canceled());
	};
	return promise;
}

/** Owns one replaceable timeout and clears it during disposal. */
export class TimeoutTimer extends AbstractDisposable {
	private timeoutHandle: ReturnType<typeof setTimeout> | undefined;

	public constructor();
	public constructor(runner: () => void, delay: number);
	public constructor(runner?: () => void, delay?: number) {
		super();
		if (runner !== undefined && delay !== undefined) this.setIfNotSet(runner, delay);
	}

	public cancel(): void {
		if (this.timeoutHandle === undefined) return;
		clearTimeout(this.timeoutHandle);
		this.timeoutHandle = undefined;
	}

	public cancelAndSet(runner: () => void, delay: number): void {
		this.assertNotDisposed();
		this.cancel();
		this.set(runner, delay);
	}

	public setIfNotSet(runner: () => void, delay: number): void {
		this.assertNotDisposed();
		if (this.timeoutHandle !== undefined) return;
		this.set(runner, delay);
	}

	protected disposeCore(): void {
		this.cancel();
	}

	private set(runner: () => void, delay: number): void {
		validateDelay(delay);
		this.timeoutHandle = setTimeout(() => {
			this.timeoutHandle = undefined;
			runner();
		}, delay);
	}
}

/** Debounces one callback and owns its pending timeout. */
export class RunOnceScheduler extends AbstractDisposable {
	private timeoutHandle: ReturnType<typeof setTimeout> | undefined;

	public constructor(
		private readonly runner: () => void,
		private defaultDelay: number,
	) {
		super();
		validateDelay(defaultDelay);
	}

	public get delay(): number {
		return this.defaultDelay;
	}

	public set delay(value: number) {
		validateDelay(value);
		this.defaultDelay = value;
	}

	public schedule(delay = this.defaultDelay): void {
		this.assertNotDisposed();
		validateDelay(delay);
		this.cancel();
		this.timeoutHandle = setTimeout(() => {
			this.timeoutHandle = undefined;
			this.runner();
		}, delay);
	}

	public cancel(): void {
		if (this.timeoutHandle === undefined) return;
		clearTimeout(this.timeoutHandle);
		this.timeoutHandle = undefined;
	}

	public isScheduled(): boolean {
		return this.timeoutHandle !== undefined;
	}

	public flush(): void {
		if (!this.isScheduled()) return;
		this.cancel();
		this.runner();
	}

	protected disposeCore(): void {
		this.cancel();
	}
}

function validateDelay(delay: number): void {
	if (!Number.isFinite(delay) || delay < 0) throw new RangeError('Timer delay must be a non-negative finite number');
}
