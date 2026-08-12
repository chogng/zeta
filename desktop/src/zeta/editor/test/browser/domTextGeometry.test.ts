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
  NodeFilter: browserEnvironment.window.NodeFilter,
})) {
  Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { createAsterDomTextRange, getAsterDomTextCaretLeft, getAsterDomTextOffsetAtClientPoint, getAsterDomTextRangeRectangles } = await import("../../browser/view/domTextGeometry.js");

test("DOM text geometry keeps one UTF-16 offset space across syntax spans", () => {
  const dom = new JSDOM("<!doctype html><body><div id=\"line\"><span id=\"text\"><span>ab</span><span>😊</span><span>cd</span></span></div></body>");
  const line = dom.window.document.querySelector<HTMLElement>("#line");
  const text = dom.window.document.querySelector<HTMLElement>("#text");
  assert.ok(line);
  assert.ok(text);

  const range = createAsterDomTextRange(text, 1, 5);
  assert.equal(range?.toString(), "b😊c");
  assert.throws(() => createAsterDomTextRange(text, 5, 1), /ordered UTF-16/);

  const textNode = text.querySelector("span")?.firstChild;
  assert.ok(textNode);
  Object.defineProperty(dom.window.document, "caretPositionFromPoint", {
    configurable: true,
    value: () => ({ offsetNode: textNode, offset: 2 }),
  });
  assert.equal(getAsterDomTextOffsetAtClientPoint(text, 20, 40), 2);
  dom.window.close();
});

test("DOM text geometry preserves browser visual rectangles for mixed-direction rendering", () => {
  const dom = new JSDOM("<!doctype html><body><div id=\"line\"><span id=\"text\">abc אבג</span></div></body>");
  const line = dom.window.document.querySelector<HTMLElement>("#line");
  const text = dom.window.document.querySelector<HTMLElement>("#text");
  assert.ok(line);
  assert.ok(text);
  Object.defineProperty(line, "getBoundingClientRect", {
    configurable: true,
    value: () => rectangle(100, 0, 0),
  });
  const createRange = dom.window.document.createRange.bind(dom.window.document);
  Object.defineProperty(dom.window.document, "createRange", {
    configurable: true,
    value: () => {
      const range = createRange();
      Object.defineProperty(range, "getClientRects", {
        configurable: true,
        value: () => [rectangle(150, 0, 20), rectangle(120, 0, 15)],
      });
      Object.defineProperty(range, "getBoundingClientRect", {
        configurable: true,
        value: () => rectangle(135, 0, 0),
      });
      return range;
    },
  });

  assert.deepEqual(getAsterDomTextRangeRectangles(text, 0, 3, line), [
    { left: 50, width: 20 },
    { left: 20, width: 15 },
  ]);
  assert.equal(getAsterDomTextCaretLeft(text, 3, line), 35);
  dom.window.close();
});

function rectangle(left: number, top: number, width: number): DOMRect {
  return { left, top, width, height: 20, right: left + width, bottom: top + 20, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}
