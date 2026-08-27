import { Disposable, MutableDisposable, type IDisposable, toDisposable } from "../common/lifecycle.js";

export interface WindowIdleOptions {
	readonly timeoutMs?: number;
}

interface ScheduledAnimationFrame {
	readonly callback: () => void;
	readonly priority: number;
	cancelled: boolean;
}

interface AnimationFrameQueue {
	frame: number | undefined;
	tasks: ScheduledAnimationFrame[];
	currentTasks: ScheduledAnimationFrame[] | undefined;
}

const animationFrameQueues = new WeakMap<Window, AnimationFrameQueue>();

/** Schedules a cancellable callback in the next animation frame. */
export function scheduleAtNextAnimationFrame(
	targetWindow: Window,
	callback: () => void,
	priority = 0,
): IDisposable {
	const queue = getAnimationFrameQueue(targetWindow);
	const task: ScheduledAnimationFrame = {
		callback,
		priority,
		cancelled: false,
	};
	queue.tasks.push(task);
	if (queue.frame === undefined) {
		const flush = (): void => {
			queue.frame = undefined;
			const tasks = queue.tasks;
			queue.tasks = [];
			queue.currentTasks = tasks;
			try {
				while (tasks.length > 0) {
					tasks.sort((left, right) => right.priority - left.priority);
					const scheduled = tasks.shift();
					if (!scheduled || scheduled.cancelled) continue;
					try {
						scheduled.callback();
					} catch (error) {
						targetWindow.queueMicrotask(() => {
							throw error;
						});
					}
				}
			} finally {
				queue.currentTasks = undefined;
			}
		};
		queue.frame = typeof targetWindow.requestAnimationFrame === "function"
			? targetWindow.requestAnimationFrame(flush)
			: targetWindow.setTimeout(flush, 16);
	}
	return toDisposable(() => {
		task.cancelled = true;
	});
}

/**
 * Runs during the current animation-frame flush when possible, otherwise
 * schedules the callback for the next frame.
 */
export function runAtThisOrScheduleAtNextAnimationFrame(
	targetWindow: Window,
	callback: () => void,
	priority = 0,
): IDisposable {
	const queue = getAnimationFrameQueue(targetWindow);
	if (!queue.currentTasks) {
		return scheduleAtNextAnimationFrame(targetWindow, callback, priority);
	}
	const task: ScheduledAnimationFrame = {
		callback,
		priority,
		cancelled: false,
	};
	queue.currentTasks.push(task);
	return toDisposable(() => {
		task.cancelled = true;
	});
}

/** Schedules layout reads before writes queued for the same frame. */
export function measure(
	targetWindow: Window,
	callback: () => void,
): IDisposable {
	return scheduleAtNextAnimationFrame(targetWindow, callback, 10_000);
}

/** Schedules layout writes after reads queued for the same frame. */
export function modify(
	targetWindow: Window,
	callback: () => void,
): IDisposable {
	return scheduleAtNextAnimationFrame(targetWindow, callback, -10_000);
}

/** Schedules cancellable work during an idle period with a timer fallback. */
export function runWhenWindowIdle(
	targetWindow: Window,
	callback: (deadline: IdleDeadline) => void,
	options: WindowIdleOptions = {},
): IDisposable {
	const idleWindow = targetWindow as Window & {
		requestIdleCallback?: (
			callback: IdleRequestCallback,
			options?: IdleRequestOptions,
		) => number;
		cancelIdleCallback?: (handle: number) => void;
	};
	if (idleWindow.requestIdleCallback && idleWindow.cancelIdleCallback) {
		const handle = idleWindow.requestIdleCallback(callback, {
			timeout: options.timeoutMs,
		});
		return toDisposable(() => idleWindow.cancelIdleCallback?.(handle));
	}

	const started = targetWindow.performance.now();
	const handle = targetWindow.setTimeout(() => callback({
		didTimeout: options.timeoutMs !== undefined,
		timeRemaining: () => Math.max(0, 50 - (targetWindow.performance.now() - started)),
	}), options.timeoutMs ?? 0);
	return toDisposable(() => targetWindow.clearTimeout(handle));
}

/** Schedules one cancellable timeout scoped to a particular browser window. */
export function disposableWindowTimeout(targetWindow: Window, callback: () => void, delayMs: number): IDisposable {
	const handle = targetWindow.setTimeout(callback, delayMs);
	return toDisposable(() => targetWindow.clearTimeout(handle));
}

/** Creates a cancellable interval scoped to a particular browser window. */
export function disposableWindowInterval(
	targetWindow: Window,
	callback: () => void,
	intervalMs: number,
): IDisposable {
	const handle = targetWindow.setInterval(callback, intervalMs);
	return toDisposable(() => targetWindow.clearInterval(handle));
}

/** Coalesces repeated schedule calls into one animation-frame callback. */
export class AnimationFrameScheduler extends Disposable {
	private readonly pending = this._register(new MutableDisposable<IDisposable>());

	constructor(
		readonly targetWindow: Window,
		readonly callback: () => void,
	) {
		super();
	}

	get scheduled(): boolean {
		return this.pending.value !== undefined;
	}

	schedule(): void {
		if (this.scheduled) return;
		this.pending.value = scheduleAtNextAnimationFrame(
			this.targetWindow,
			() => {
				this.pending.clear();
				this.callback();
			},
		);
	}

	cancel(): void {
		this.pending.clear();
	}
}

function getAnimationFrameQueue(
	targetWindow: Window,
): AnimationFrameQueue {
	let queue = animationFrameQueues.get(targetWindow);
	if (!queue) {
		queue = { frame: undefined, tasks: [], currentTasks: undefined };
		animationFrameQueues.set(targetWindow, queue);
	}
	return queue;
}
