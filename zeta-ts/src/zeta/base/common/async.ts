import { CancellationError } from './errors.js';

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
		rejectPromise(new CancellationError());
	};
	return promise;
}
