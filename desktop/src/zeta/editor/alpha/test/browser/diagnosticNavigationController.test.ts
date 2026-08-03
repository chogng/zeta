import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { TextDecorationCollection } from "../../common/decoration.js";
import { EditorSelectionController } from "../../common/editorSelectionController.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic } from "../../language/common/languageResults.js";
import { TextSelection, TextSelectionSet } from "../../common/selection.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";
import { TrackedRangeStickiness } from "../../common/trackedRange.js";

const environment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({ window: environment.window, document: environment.window.document, Node: environment.window.Node, Element: environment.window.Element, HTMLElement: environment.window.HTMLElement, Event: environment.window.Event, KeyboardEvent: environment.window.KeyboardEvent })) Object.defineProperty(globalThis, name, { configurable: true, value });
const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaDiagnosticNavigationController } = await import("../../language/browser/diagnosticNavigationController.js");

test("F8 navigates current diagnostics in both directions", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("one\ntwo\nthree");
  using selections = new EditorSelectionController(model, TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))));
  using diagnostics = new TextDecorationCollection<LanguageDiagnostic>(model);
  diagnostics.add({ range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 2)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: diagnostic("first") });
  diagnostics.add({ range: TextRange.from(TextPosition.at(2, 1), TextPosition.at(2, 3)), stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: diagnostic("last") });
  using viewport = new AlphaEditorViewport({ container, model, lineHeight: 20, textMeasurer: new FixedTextMeasurer(), selectionController: selections });
  const input = dom.window.document.createElement("textarea");
  container.append(input);
  using controller = new AlphaDiagnosticNavigationController(input, viewport, selections, diagnostics);
  const next = key(dom.window, false); input.dispatchEvent(next);
  assert.equal(next.defaultPrevented, true); assert.deepEqual(selections.selections.primary.range, TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 2)));
  assert.equal(viewport.element.querySelector(".zeta-alpha-editor-accessibility-status")?.textContent, "warning: first");
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
class FixedTextMeasurer implements AlphaTextMeasurer { readonly horizontalPadding = 24; readonly contentLeftPadding = 12; refresh(): boolean { return false; } measureLineWidth(text: string): number { return text.length * 10; } }
