import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../../../base/common/event.js";
import { CompositeEditorLineGutterDecoration, type EditorLineGutterDecoration } from "../../browser/viewparts/margin/lineGutterDecoration.js";
import { h } from "../../../base/browser/dom.js";

test("gutter decorations compose into independent ordered slots", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  using first = new TestDecoration("first");
  using second = new TestDecoration("second");
  using composite = new CompositeEditorLineGutterDecoration([first, second]);
  const element = composite.create(dom.window.document);

  composite.project(element, 6, true);

  assert.equal(composite.width, 40);
  assert.deepEqual([...element.querySelectorAll("button")].map(button => ({ text: button.textContent, line: button.dataset.line })), [{ text: "first", line: "6" }, { text: "second", line: "6" }]);
  dom.window.close();
});

class TestDecoration implements EditorLineGutterDecoration {
  private readonly emitter = new Emitter<void>();
  readonly onDidChange = this.emitter.event;
  constructor(private readonly label: string) {}
  create(ownerDocument: Document): HTMLElement { const button = h(ownerDocument, "button"); button.textContent = this.label; return button; }
  project(element: HTMLElement, logicalLineIndex: number): void { element.dataset.line = String(logicalLineIndex); }
  dispose(): void { this.emitter.dispose(); }
  [Symbol.dispose](): void { this.dispose(); }
}
