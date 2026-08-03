import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AlphaDecorationPresentation, createAlphaDecorationSource } from "../../browser/decorationPresentation.js";
import { AlphaDiagnosticHoverController } from "../../language/browser/diagnosticHoverController.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { TextDecorationCollection } from "../../common/decoration.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";
import { TrackedRangeStickiness } from "../../common/trackedRange.js";

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return [...text].length * 10;
  }
}

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

test("Diagnostic hover presents current gutter-marker messages and hides on pointer exit", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = dom.window.document.querySelector<HTMLElement>("main")!;
  using model = new TextModel("const value");
  using decorations = new TextDecorationCollection<string>(model);
  decorations.add({
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: "Use let instead",
  });
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    decorationSources: [createAlphaDecorationSource(
      decorations,
      () => AlphaDecorationPresentation.WarningUnderline,
      decoration => decoration.metadata,
    )],
  });
  using controller = new AlphaDiagnosticHoverController(viewport);
  viewport.layout({ width: 160, height: 20 });
  const marker = viewport.element.querySelector<HTMLElement>(".zeta-alpha-editor-diagnostic-marker")!;
  marker.dispatchEvent(new dom.window.Event("pointerover", { bubbles: true }));
  const hover = dom.window.document.body.querySelector<HTMLElement>(".zeta-alpha-editor-diagnostic-hover")!;
  assert.equal(hover.hidden, false);
  assert.equal(hover.textContent, "Use let instead");

  marker.dispatchEvent(new dom.window.Event("pointerout", { bubbles: true }));
  assert.equal(hover.hidden, true);
  dom.window.close();
});
