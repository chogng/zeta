import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import type { ITaskQueue } from './taskQueue.js';

interface TaskDeadline {
	timeRemaining(): number;
}

/** Queue used by the current renderer before it is created through the service container. */
export class WindowIdleTaskQueue extends Disposable implements ITaskQueue {
	private readonly tasks: (() => boolean | void)[] = [];
	private callbackIdentifier: number | undefined;
	private taskIndex = 0;

	constructor(private readonly ownerWindow: Window) {
		super();
		this._register(toDisposable(() => this.clear()));
	}

	enqueue(task: () => boolean | void): void {
		this.tasks.push(task);
		this.start();
	}

	flush(): void {
		while (this.taskIndex < this.tasks.length) {
			if (!this.tasks[this.taskIndex]()) this.taskIndex++;
		}
		this.clear();
	}

	clear(): void {
		if (this.callbackIdentifier !== undefined) this.cancelCallback(this.callbackIdentifier);
		this.callbackIdentifier = undefined;
		this.taskIndex = 0;
		this.tasks.length = 0;
	}

	private start(): void {
		if (this.callbackIdentifier === undefined) {
			this.callbackIdentifier = this.requestCallback(deadline => this.process(deadline));
		}
	}

	private process(deadline: TaskDeadline): void {
		this.callbackIdentifier = undefined;
		while (this.taskIndex < this.tasks.length && deadline.timeRemaining() > 0) {
			if (!this.tasks[this.taskIndex]()) this.taskIndex++;
		}
		if (this.taskIndex < this.tasks.length) this.start();
		else this.clear();
	}

	private requestCallback(callback: (deadline: TaskDeadline) => void): number {
		const requestIdleCallback = (this.ownerWindow as Window & { requestIdleCallback?: (callback: (deadline: TaskDeadline) => void) => number }).requestIdleCallback;
		if (!requestIdleCallback) return this.ownerWindow.setTimeout(() => callback({ timeRemaining: () => 1 }), 0);
		return requestIdleCallback.call(this.ownerWindow, callback);
	}

	private cancelCallback(identifier: number): void {
		const cancelIdleCallback = (this.ownerWindow as Window & { cancelIdleCallback?: (identifier: number) => void }).cancelIdleCallback;
		if (cancelIdleCallback) cancelIdleCallback.call(this.ownerWindow, identifier);
		else this.ownerWindow.clearTimeout(identifier);
	}
}
