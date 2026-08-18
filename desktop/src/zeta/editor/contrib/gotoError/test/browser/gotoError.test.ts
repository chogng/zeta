import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type TextMeasurer } from "../../../../browser/view/fontMetrics.js";
import { TextDecorationCollection } from "../../../../common/model/decorationCollection.js";
import { EditorSelectionController } from "../../../../common/cursor/editorSelectionController.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../../../common/languages/languageResults.js";
import { TextSelection, TextSelectionSet } from "../../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";
import { TextModel } from "../../../../common/model/textModel.js";
import { TrackedRangeStickiness } from "../../../../common/model/trackedRange.js";
import { h } from "../../../../../base/browser/dom.js";

const environment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({ window: environment.window, document: environment.window.document, Node: environment.window.Node, Element: environment.window.Element, HTMLElement: environment.window.HTMLElement, Event: environment.window.Event, KeyboardEvent: environment.window.KeyboardEvent })) Object.defineProperty(globalThis, name, { configurable: true, value });
const { EditorViewport } = await import("../../../../browser/view/editorViewport.js");
const { DiagnosticNavigationController } = await import("../../browser/gotoError.js");

test("F8 navigates current diagnostics in both directions", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("one\ntwo\nthree");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using diagnostics = new TextDecorationCollection<LanguageDiagnostic>(model);
  diagnostics.add({ range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 2)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: diagnostic("first") });
  diagnostics.add({ range: TextRange.from(TextPosition.at(2, 1), TextPosition.at(2, 3)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: diagnostic("last") });
  using viewport = new EditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  const input = h(dom.window.document, "textarea");
  container.append(input);
  using controller = new DiagnosticNavigationController(input, viewport, selections, diagnostics);
  const next = key(dom.window, false); input.dispatchEvent(next);
  assert.equal(next.defaultPrevented, true); assert.deepEqual(selections.selections.primary.range, TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 2)));
  assert.equal(viewport.element.querySelector(".aster-editor-accessibility-status")?.textContent, "warning: first");
  input.dispatchEvent(key(dom.window, false));
  assert.deepEqual(selections.selections.primary.range, TextRange.from(TextPosition.at(2, 1), TextPosition.at(2, 3)));
  const previous = key(dom.window, true);
  assert.equal(previous.shiftKey, true);
  input.dispatchEvent(previous);
  assert.equal(previous.defaultPrevented, true);
  assert.deepEqual(selections.selections.primary.range, TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 2)));
  dom.window.close();
});

function diagnostic(message: string): LanguageDiagnostic { return { range: TextRange.emptyAt(TextPosition.at(0, 0)), severity: LanguageDiagnosticSeverity.Warning, message }; }
function key(window: typeof environment.window, shiftKey: boolean): KeyboardEvent { return new window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: "F8", shiftKey }) as unknown as KeyboardEvent; }
class FixedTextMeasurer implements TextMeasurer { readonly horizontalPadding = 24; readonly contentLeftPadding = 12; refresh(): boolean { return false; } measureLineWidth(text: string): number { return text.length * 10; } }
