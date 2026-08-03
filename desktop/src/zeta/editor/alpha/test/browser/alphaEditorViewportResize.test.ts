import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { TextModel } from "../../common/textModel.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");

test.after(() => browserEnvironment.window.close());

test("Alpha viewport resize observations use the scrollable client area", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  let resizeListener: ResizeObserverCallback | undefined;
  class TestResizeObserver {
    constructor(listener: ResizeObserverCallback) {
      resizeListener = listener;
    }

    observe(): void {}
    disconnect(): void {}
  }
  Object.defineProperty(dom.window, "ResizeObserver", { configurable: true, value: TestResizeObserver });
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel();
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20 });
  Object.defineProperties(viewport.element, {
    clientWidth: { configurable: true, value: 383 },
    clientHeight: { configurable: true, value: 62 },
  });

  resizeListener?.([{ contentRect: { width: 383.3875, height: 46.7875 } } as ResizeObserverEntry], {} as ResizeObserver);

  assert.deepEqual(viewport.viewportLayout.viewportSize, { width: 383, height: 62 });
  assert.equal(viewport.element.classList.contains("horizontally-scrollable"), false);
  assert.equal(viewport.element.classList.contains("vertically-scrollable"), false);
  assert.equal(requiredElement<HTMLElement>(viewport.element, ".zeta-alpha-editor-content").style.width, "383px");
  assert.equal(requiredElement<HTMLElement>(viewport.element, ".zeta-alpha-editor-content").style.height, "62px");
  dom.window.close();
});

test("Alpha viewport enables scrollbars only for model-backed overflow", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  dom.window.HTMLCanvasElement.prototype.getContext = () => null;
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel(`${"x".repeat(100)}\nsecond line`);
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20 });

  viewport.layout({ width: 50, height: 20 });

  assert.equal(viewport.element.classList.contains("horizontally-scrollable"), true);
  assert.equal(viewport.element.classList.contains("vertically-scrollable"), true);
  dom.window.close();
});

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
  const element = root.querySelector<T>(selector);
  assert.ok(element);
  return element;
}
