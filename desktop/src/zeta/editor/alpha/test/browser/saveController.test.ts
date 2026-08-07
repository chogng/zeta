import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
  KeyboardEvent: browserEnvironment.window.KeyboardEvent,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { SaveController } = await import("../../browser/saveController.js");

test.after(() => browserEnvironment.window.close());

test("Save shortcut prevents browser save and does not overlap writes", async () => {
  const dom = new JSDOM("<!doctype html><body><textarea></textarea></body>");
  const input = dom.window.document.querySelector<HTMLTextAreaElement>("textarea")!;
  const pending = deferred<void>();
  let saves = 0;
  using controller = new SaveController(input, {
    save: async () => {
      saves += 1;
      await pending.promise;
    },
  });

  const first = keydown(dom.window, "s", { ctrlKey: true });
  input.dispatchEvent(first);
  assert.equal(first.defaultPrevented, true);
  await nextTask();
  assert.equal(saves, 1);

  const repeated = keydown(dom.window, "s", { metaKey: true });
  input.dispatchEvent(repeated);
  assert.equal(repeated.defaultPrevented, true);
  await nextTask();
  assert.equal(saves, 1);

  pending.resolve();
  await nextTask();
  const next = keydown(dom.window, "s", { ctrlKey: true });
  input.dispatchEvent(next);
  await nextTask();
  assert.equal(saves, 2);
  dom.window.close();
});

test("Save shortcut reports successful completion", async () => {
  const dom = new JSDOM("<!doctype html><body><textarea></textarea></body>");
  const input = dom.window.document.querySelector<HTMLTextAreaElement>("textarea")!;
  let successes = 0;
  using controller = new SaveController(input, {
    save: async () => {},
    onSaveSuccess: () => {
      successes += 1;
    },
  });

  input.dispatchEvent(keydown(dom.window, "s", { ctrlKey: true }));
  await nextTask();
  assert.equal(successes, 1);
  dom.window.close();
});

function keydown(targetWindow: typeof browserEnvironment.window, key: string, options: { readonly ctrlKey?: boolean; readonly metaKey?: boolean } = {}): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
    ctrlKey: options.ctrlKey,
    metaKey: options.metaKey,
  }) as unknown as KeyboardEvent;
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(resolver => {
    resolve = resolver;
  });
  return { promise, resolve };
}

function nextTask(): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, 0));
}
