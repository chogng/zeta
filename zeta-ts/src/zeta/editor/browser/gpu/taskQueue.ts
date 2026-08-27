import { Disposable, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';

export interface ITaskQueue extends IDisposable {
	enqueue(task: () => boolean | void): void;
	flush(): void;
	clear(): void;
}

interface TaskDeadline {
	timeRemaining(): number;
}

abstract class TaskQueue extends Disposable implements ITaskQueue {
	private readonly tasks: (() => boolean | void)[] = [];
	private callbackIdentifier: number | undefined;
	private taskIndex = 0;

	constructor() {
		super();
		this._register(toDisposable(() => this.clear()));
	}

	public enqueue(task: () => boolean | void): void {
		this.tasks.push(task);
		this.start();
	}

	public flush(): void {
		while (this.taskIndex < this.tasks.length) {
			if (!this.tasks[this.taskIndex]()) this.taskIndex += 1;
		}
		this.clear();
	}

	public clear(): void {
		if (this.callbackIdentifier !== undefined) this.cancelCallback(this.callbackIdentifier);
		this.callbackIdentifier = undefined;
		this.taskIndex = 0;
		this.tasks.length = 0;
	}

	protected abstract requestCallback(callback: (deadline: TaskDeadline) => void): number;
	protected abstract cancelCallback(identifier: number): void;

	private start(): void {
		if (this.callbackIdentifier === undefined) this.callbackIdentifier = this.requestCallback(deadline => this.process(deadline));
	}

	private process(deadline: TaskDeadline): void {
		this.callbackIdentifier = undefined;
		while (this.taskIndex < this.tasks.length && deadline.timeRemaining() > 0) {
			if (!this.tasks[this.taskIndex]()) this.taskIndex += 1;
		}
		if (this.taskIndex < this.tasks.length) this.start();
		else this.clear();
	}
}

export class PriorityTaskQueue extends TaskQueue {
	constructor(private readonly ownerWindow: Window) { super(); }

	protected requestCallback(callback: (deadline: TaskDeadline) => void): number {
		return this.ownerWindow.setTimeout(() => callback({ timeRemaining: () => 1 }), 0);
	}

	protected cancelCallback(identifier: number): void {
		this.ownerWindow.clearTimeout(identifier);
	}
}

export const IdleTaskQueue = class IdleTaskQueue extends TaskQueue {
	constructor(private readonly ownerWindow: Window) { super(); }

	protected requestCallback(callback: (deadline: TaskDeadline) => void): number {
		const requestIdleCallback = (this.ownerWindow as Window & { requestIdleCallback?: (callback: (deadline: TaskDeadline) => void) => number }).requestIdleCallback;
		if (!requestIdleCallback) return this.ownerWindow.setTimeout(() => callback({ timeRemaining: () => 1 }), 0);
		return requestIdleCallback.call(this.ownerWindow, callback);
	}

	protected cancelCallback(identifier: number): void {
		const cancelIdleCallback = (this.ownerWindow as Window & { cancelIdleCallback?: (identifier: number) => void }).cancelIdleCallback;
		if (cancelIdleCallback) cancelIdleCallback.call(this.ownerWindow, identifier);
		else this.ownerWindow.clearTimeout(identifier);
	}
};

export class DebouncedIdleTask {
	private readonly queue: ITaskQueue;

	constructor(ownerWindow: Window) {
		this.queue = new IdleTaskQueue(ownerWindow);
	}

	public set(task: () => boolean | void): void {
		this.queue.clear();
		this.queue.enqueue(task);
	}

	public flush(): void {
		this.queue.flush();
	}
}
