import assert from "node:assert/strict";
import test from "node:test";
import { disposableWindowTimeout, measure, modify, runAtThisOrScheduleAtNextAnimationFrame, scheduleAtNextAnimationFrame } from "../../browser/scheduler.js";

test("animation-frame scheduling orders reads, ordinary work, and writes", () => {
  const targetWindow = new TestAnimationWindow();
  const order: string[] = [];
  modify(targetWindow.value, () => order.push("modify"));
  scheduleAtNextAnimationFrame(targetWindow.value, () => order.push("ordinary"));
  measure(targetWindow.value, () => order.push("measure"));

  targetWindow.flush();

  assert.deepEqual(order, ["measure", "ordinary", "modify"]);
});

test("current-frame scheduling joins an active flush by priority", () => {
  const targetWindow = new TestAnimationWindow();
  const order: string[] = [];
  scheduleAtNextAnimationFrame(targetWindow.value, () => {
    order.push("first");
    runAtThisOrScheduleAtNextAnimationFrame(targetWindow.value, () => order.push("same-frame-high"), 100);
    runAtThisOrScheduleAtNextAnimationFrame(targetWindow.value, () => order.push("same-frame-low"), -100);
  });
  scheduleAtNextAnimationFrame(targetWindow.value, () => order.push("queued"));

  targetWindow.flush();

  assert.deepEqual(order, ["first", "same-frame-high", "queued", "same-frame-low"]);
  assert.equal(targetWindow.pendingFrames, 0);
});

test("current-frame scheduling remains cancellable", () => {
  const targetWindow = new TestAnimationWindow();
  let called = false;
  scheduleAtNextAnimationFrame(targetWindow.value, () => {
    const registration = runAtThisOrScheduleAtNextAnimationFrame(targetWindow.value, () => called = true);
    registration.dispose();
  });

  targetWindow.flush();

  assert.equal(called, false);
});

test("animation-frame scheduling falls back to an owned window timer", () => {
  const targetWindow = new TestTimerWindow();
  let calls = 0;
  scheduleAtNextAnimationFrame(targetWindow.value, () => calls += 1);

  assert.equal(targetWindow.pendingTimers, 1);
  targetWindow.flush();
  assert.equal(calls, 1);
});

test("window timeouts remain cancellable", () => {
  const targetWindow = new TestTimerWindow();
  let called = false;
  const registration = disposableWindowTimeout(targetWindow.value, () => called = true, 10);

  registration.dispose();
  targetWindow.flush();
  assert.equal(called, false);
});

class TestAnimationWindow {
  private nextHandle = 1;
  private callbacks = new Map<number, FrameRequestCallback>();

  readonly value = {
    requestAnimationFrame: (callback: FrameRequestCallback): number => {
      const handle = this.nextHandle++;
      this.callbacks.set(handle, callback);
      return handle;
    },
    queueMicrotask,
  } as unknown as Window;

  get pendingFrames(): number {
    return this.callbacks.size;
  }

  flush(): void {
    const callbacks = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of callbacks) callback(0);
  }
}

class TestTimerWindow {
  private nextHandle = 1;
  private callbacks = new Map<number, () => void>();

  readonly value = {
    setTimeout: (callback: () => void): number => {
      const handle = this.nextHandle++;
      this.callbacks.set(handle, callback);
      return handle;
    },
    clearTimeout: (handle: number): void => {
      this.callbacks.delete(handle);
    },
    queueMicrotask,
  } as unknown as Window;

  get pendingTimers(): number {
    return this.callbacks.size;
  }

  flush(): void {
    const callbacks = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of callbacks) callback();
  }
}
