import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { TextRange } from "../../../common/core/text.js";
import { type DiffComputationRequest, type IDiffComputationService } from "../../../common/diff/diffComputationService.js";
import { LineDiffKind, type LineDiff } from "../../../common/diff/lineDiff.js";
import { TextModel } from "../../../common/model/textModel.js";

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

const { DiffEditorWidget } = await import("../../../browser/widget/diffEditor/diffEditorWidget.js");
const { DiffModel } = await import("../../../common/diff/diffModel.js");

test("DiffEditorWidget presents side-by-side changed lines and inline ranges", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using original = new TextModel("same\nold value\nremoved\ntail");
  using modified = new TextModel("same\nnew value\nadded\ntail");
  using computationService = new WidgetTestDiffComputationService();
  using model = new DiffModel({ original, modified, computationService });
  await waitForReady(model);
  using editor = new DiffEditorWidget({ container, model, lineHeight: 20 });
  editor.layout({ width: 400, height: 80 });

  const rows = [...editor.element.querySelectorAll<HTMLElement>(".zeta-alpha-diff-row")];
  assert.equal(rows.length, 4);
  assert.equal(rows[0]?.classList.contains("unchanged"), true);
  assert.equal(rows[1]?.classList.contains("modified"), true);
  assert.equal(rows[1]?.querySelector(".zeta-alpha-diff-cell.original")?.textContent, "2old value");
  assert.equal(rows[1]?.querySelector(".zeta-alpha-diff-cell.modified")?.textContent, "2new value");
  assert.equal(rows[1]?.querySelectorAll(".zeta-alpha-diff-inline.removed").length, 1);
  assert.equal(rows[1]?.querySelectorAll(".zeta-alpha-diff-inline.added").length, 1);
  assert.equal(editor.nextChange(), 1);
  assert.equal(editor.currentChangeRow, 1);
  assert.equal(editor.element.querySelector(".zeta-alpha-diff-row.active")?.classList.contains("modified"), true);
  assert.equal(editor.previousChange(), 2);
  const next = new dom.window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: "F7" });
  editor.element.dispatchEvent(next);
  assert.equal(next.defaultPrevented, true);
  assert.equal(editor.currentChangeRow, 1);
  assert.match(editor.element.querySelector(".zeta-alpha-diff-editor-accessibility-status")?.textContent ?? "", /Change 1 of 2/);
  dom.window.close();
});

test("DiffEditorWidget refreshes on either source model and virtualizes diff rows", async () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using original = new TextModel(lines("old", 100));
  using modified = new TextModel(lines("new", 100));
  using computationService = new WidgetTestDiffComputationService();
  using model = new DiffModel({ original, modified, computationService });
  await waitForReady(model);
  using editor = new DiffEditorWidget({ container, model, lineHeight: 20, overscanRowCount: 1 });
  editor.layout({ width: 400, height: 40 });

  assert.equal(editor.element.querySelectorAll(".zeta-alpha-diff-row").length, 3);
  editor.revealModifiedLine(80);
  const firstVisibleRow = editor.element.querySelector<HTMLElement>(".zeta-alpha-diff-row");
  assert.equal(firstVisibleRow?.style.height, "20px");
  assert.ok(editor.element.scrollTop > 0);

  modified.applyEdits([{
    range: TextRange.from(modified.positionAt(0), modified.positionAt(modified.getText().length)),
    text: "same",
  }]);
  await waitForReady(model);
  assert.equal(editor.diff?.rows.length, 100);
  editor.element.scrollTop = 0;
  editor.element.dispatchEvent(new dom.window.Event("scroll"));
  assert.equal(editor.element.querySelector(".zeta-alpha-diff-row")?.classList.contains("modified"), true);
  dom.window.close();
});

function lines(prefix: string, count: number): string {
  return Array.from({ length: count }, (_, index) => `${prefix} ${index}`).join("\n");
}

class WidgetTestDiffComputationService implements IDiffComputationService {
  async compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
    signal.throwIfAborted();
    const originalLines = request.original.text.split("\n");
    const modifiedLines = request.modified.text.split("\n");
    const rows = Array.from({ length: Math.max(originalLines.length, modifiedLines.length) }, (_, index) => {
      const original = originalLines[index];
      const modified = modifiedLines[index];
      if (original === undefined) {
        return Object.freeze({ kind: LineDiffKind.Added, modifiedLineIndex: index, originalChanges: Object.freeze([]), modifiedChanges: Object.freeze([]) });
      }
      if (modified === undefined) {
        return Object.freeze({ kind: LineDiffKind.Removed, originalLineIndex: index, originalChanges: Object.freeze([]), modifiedChanges: Object.freeze([]) });
      }
      if (original === modified) {
        return Object.freeze({ kind: LineDiffKind.Unchanged, originalLineIndex: index, modifiedLineIndex: index, originalChanges: Object.freeze([]), modifiedChanges: Object.freeze([]) });
      }
      return Object.freeze({
        kind: LineDiffKind.Modified,
        originalLineIndex: index,
        modifiedLineIndex: index,
        originalChanges: Object.freeze(original.length === 0 ? [] : [{ startColumn: 0, endColumn: original.length }]),
        modifiedChanges: Object.freeze(modified.length === 0 ? [] : [{ startColumn: 0, endColumn: modified.length }]),
      });
    });
    return Object.freeze({ rows: Object.freeze(rows), hunks: Object.freeze([]) });
  }

  dispose(): void {}

  [Symbol.dispose](): void {
    this.dispose();
  }
}

function requiredElement<T extends Element>(ownerDocument: Document, selector: string): T {
  const element = ownerDocument.querySelector<T>(selector);
  if (!element) throw new Error(`Missing ${selector}`);
  return element;
}

function waitForReady(model: InstanceType<typeof DiffModel>): Promise<void> {
  if (model.state.kind === "ready") return Promise.resolve();
  return new Promise((resolve, reject) => {
    const listener = model.onDidChange(state => {
      if (state.kind === "loading") return;
      listener.dispose();
      if (state.kind === "error") reject(state.error);
      else resolve();
    });
  });
}
