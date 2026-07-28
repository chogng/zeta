import { DisposableOwner, DisposableSlot, toDisposable, } from "../common/lifecycle.js";
const animationFrameQueues = new WeakMap();
/** Schedules a cancellable callback in the next animation frame. */
export function scheduleAtNextAnimationFrame(targetWindow, callback, priority = 0) {
    const queue = getAnimationFrameQueue(targetWindow);
    const task = {
        callback,
        priority,
        cancelled: false,
    };
    queue.tasks.push(task);
    if (queue.frame === undefined) {
        queue.frame = targetWindow.requestAnimationFrame(() => {
            queue.frame = undefined;
            const tasks = queue.tasks;
            queue.tasks = [];
            tasks.sort((left, right) => right.priority - left.priority);
            for (const scheduled of tasks) {
                if (scheduled.cancelled)
                    continue;
                try {
                    scheduled.callback();
                }
                catch (error) {
                    targetWindow.queueMicrotask(() => {
                        throw error;
                    });
                }
            }
        });
    }
    return toDisposable(() => {
        task.cancelled = true;
    });
}
/** Schedules layout reads before writes queued for the same frame. */
export function measure(targetWindow, callback) {
    return scheduleAtNextAnimationFrame(targetWindow, callback, 10_000);
}
/** Schedules layout writes after reads queued for the same frame. */
export function modify(targetWindow, callback) {
    return scheduleAtNextAnimationFrame(targetWindow, callback, -10_000);
}
/** Schedules cancellable work during an idle period with a timer fallback. */
export function runWhenWindowIdle(targetWindow, callback, options = {}) {
    const idleWindow = targetWindow;
    if (idleWindow.requestIdleCallback && idleWindow.cancelIdleCallback) {
        const handle = idleWindow.requestIdleCallback(callback, {
            timeout: options.timeoutMs,
        });
        return toDisposable(() => idleWindow.cancelIdleCallback?.(handle));
    }
    const started = performance.now();
    const handle = targetWindow.setTimeout(() => callback({
        didTimeout: options.timeoutMs !== undefined,
        timeRemaining: () => Math.max(0, 50 - (performance.now() - started)),
    }), options.timeoutMs ?? 0);
    return toDisposable(() => targetWindow.clearTimeout(handle));
}
/** Creates a cancellable interval scoped to a particular browser window. */
export function disposableWindowInterval(targetWindow, callback, intervalMs) {
    const handle = targetWindow.setInterval(callback, intervalMs);
    return toDisposable(() => targetWindow.clearInterval(handle));
}
/** Coalesces repeated schedule calls into one animation-frame callback. */
export class AnimationFrameScheduler extends DisposableOwner {
    targetWindow;
    callback;
    #pending = this.own(new DisposableSlot());
    constructor(targetWindow, callback) {
        super();
        this.targetWindow = targetWindow;
        this.callback = callback;
    }
    get scheduled() {
        return this.#pending.value !== undefined;
    }
    schedule() {
        if (this.scheduled)
            return;
        this.#pending.replace(scheduleAtNextAnimationFrame(this.targetWindow, () => {
            this.#pending.clear();
            this.callback();
        }));
    }
    cancel() {
        this.#pending.clear();
    }
}
function getAnimationFrameQueue(targetWindow) {
    let queue = animationFrameQueues.get(targetWindow);
    if (!queue) {
        queue = { frame: undefined, tasks: [] };
        animationFrameQueues.set(targetWindow, queue);
    }
    return queue;
}
