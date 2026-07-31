import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AlphaDecorationPresentation, createAlphaDecorationSource } from "../../browser/decorationPresentation.js";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { createAlphaLanguageDiagnosticSource, resolveAlphaLanguageDiagnosticPresentation } from "../../browser/languageDiagnosticPresentation.js";
import { TextDecorationCollection } from "../../common/decoration.js";
import { LanguageDiagnosticDecorationBridge } from "../../common/languageDiagnosticDecorations.js";
import { LanguageResultAcceptance } from "../../common/languageResultStore.js";
import { LanguageDiagnosticSeverity, createLanguageDiagnosticStore } from "../../common/languageResults.js";
import { TextPosition, TextRange } from "../../common/text.js";
import { TextModel } from "../../common/textModel.js";
import { TrackedRangeStickiness } from "../../common/trackedRange.js";

const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
  window: browserEnvironment.window,
  document: browserEnvironment.window.document,
  Node: browserEnvironment.window.Node,
  Element: browserEnvironment.window.Element,
  HTMLElement: browserEnvironment.window.HTMLElement,
  Event: browserEnvironment.window.Event,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import(
  "../../browser/alphaEditorViewport.js"
);

test("Decoration sources project, update, and follow tracked model ranges", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abcd\nefgh\nij");
  using matches = new TextDecorationCollection<string>(model);
  using diagnostics = new TextDecorationCollection<"error" | "warning">(model);
  const matchId = matches.add({
    range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(1, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: "match",
  });
  const diagnosticId = diagnostics.add({
    range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: "error",
  });
  let matchResolutionCount = 0;
  const matchSource = createAlphaDecorationSource(
    matches,
    () => {
      matchResolutionCount += 1;
      return AlphaDecorationPresentation.SearchMatch;
    },
  );
  const diagnosticSource = createAlphaDecorationSource(
    diagnostics,
    decoration => decoration.metadata === "error"
      ? AlphaDecorationPresentation.ErrorUnderline
      : AlphaDecorationPresentation.WarningUnderline,
  );
  const viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    decorationSources: [matchSource, diagnosticSource],
  });
  viewport.layout({ width: 200, height: 60 });
  viewport.scrollTo({ left: 0, top: 0 });
  assert.equal(matchResolutionCount, 1);

  assert.deepEqual(decorationElements(viewport.element).map(element => ({
    id: element.dataset.decorationId,
    presentation: element.classList[1],
    lineIndex: element.parentElement?.parentElement?.dataset.lineIndex,
    left: element.style.left,
    width: element.style.width,
  })), [{
    id: String(matchId),
    presentation: AlphaDecorationPresentation.SearchMatch,
    lineIndex: "0",
    left: "48px",
    width: "40px",
  }, {
    id: String(matchId),
    presentation: AlphaDecorationPresentation.SearchMatch,
    lineIndex: "1",
    left: "38px",
    width: "20px",
  }, {
    id: String(diagnosticId),
    presentation: AlphaDecorationPresentation.ErrorUnderline,
    lineIndex: "2",
    left: "38px",
    width: "20px",
  }]);

  diagnostics.update(diagnosticId, {
    range: TextRange.from(TextPosition.at(1, 1), TextPosition.at(1, 3)),
    stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
    metadata: "warning",
  });
  const warning = requiredElement<HTMLElement>(
    viewport.element,
    `.zeta-alpha-editor-decoration[data-decoration-id="${diagnosticId}"]`,
  );
  assert.equal(
    warning.classList.contains(AlphaDecorationPresentation.WarningUnderline),
    true,
  );
  assert.equal(warning.parentElement?.parentElement?.dataset.lineIndex, "1");
  assert.equal(warning.style.left, "48px");
  assert.equal(warning.style.width, "20px");
  assert.equal(matchResolutionCount, 1);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X\n",
  }]);

  const trackedMatch = decorationElements(viewport.element).filter(
    element => element.dataset.decorationId === String(matchId),
  );
  assert.deepEqual(
    trackedMatch.map(
      element => element.parentElement?.parentElement?.dataset.lineIndex,
    ),
    ["1", "2"],
  );
  assert.equal(matchResolutionCount, 2);
  viewport.dispose();
  assert.equal(matches.size, 1);
  assert.equal(diagnostics.size, 1);

  dom.window.close();
});

test("Versioned diagnostics project named severity underlines and invalidate", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement(dom.window.document, "main");
  using model = new TextModel("abcd\nefgh\nijkl\nmnop");
  using store = createLanguageDiagnosticStore(model);
  using bridge = new LanguageDiagnosticDecorationBridge(store);
  const source = createAlphaLanguageDiagnosticSource(bridge.decorations);
  const viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    decorationSources: [source],
  });
  viewport.layout({ width: 200, height: 80 });
  assert.equal(store.accept({
    requestId: 1,
    textModel: model,
    modelVersion: 1,
    value: {
      diagnostics: [
        {
          range: TextRange.from(TextPosition.at(0, 1), TextPosition.at(0, 3)),
          severity: LanguageDiagnosticSeverity.Error,
          message: "error",
        },
        {
          range: TextRange.from(TextPosition.at(1, 0), TextPosition.at(1, 2)),
          severity: LanguageDiagnosticSeverity.Warning,
          message: "warning",
        },
        {
          range: TextRange.from(TextPosition.at(2, 0), TextPosition.at(2, 2)),
          severity: LanguageDiagnosticSeverity.Information,
          message: "information",
        },
        {
          range: TextRange.emptyAt(TextPosition.at(3, 1)),
          severity: LanguageDiagnosticSeverity.Hint,
          message: "hint",
        },
      ],
    },
  }), LanguageResultAcceptance.Applied);

  assert.deepEqual(decorationElements(viewport.element).map(element => ({
    presentation: element.classList[1],
    lineIndex: element.parentElement?.parentElement?.dataset.lineIndex,
  })), [{
    presentation: AlphaDecorationPresentation.ErrorUnderline,
    lineIndex: "0",
  }, {
    presentation: AlphaDecorationPresentation.WarningUnderline,
    lineIndex: "1",
  }]);
  assert.equal(
    resolveAlphaLanguageDiagnosticPresentation(
      LanguageDiagnosticSeverity.Information,
    ),
    undefined,
  );
  assert.equal(
    resolveAlphaLanguageDiagnosticPresentation(LanguageDiagnosticSeverity.Hint),
    undefined,
  );
  assert.throws(
    () => resolveAlphaLanguageDiagnosticPresentation(
      "fatal" as LanguageDiagnosticSeverity,
    ),
    /Unknown language diagnostic severity/,
  );

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X",
  }]);
  assert.deepEqual(decorationElements(viewport.element), []);
  assert.equal(store.result, undefined);

  viewport.dispose();
  assert.equal(store.accept({
    requestId: 2,
    textModel: model,
    modelVersion: 2,
    value: {
      diagnostics: [{
        range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
        severity: LanguageDiagnosticSeverity.Error,
        message: "after viewport",
      }],
    },
  }), LanguageResultAcceptance.Applied);
  assert.equal(bridge.decorations.size, 1);
  dom.window.close();
});

function requiredElement<T extends Element = HTMLElement>(
  container: ParentNode,
  selector: string,
): T {
  const element = container.querySelector<T>(selector);
  assert.ok(element, `Expected ${selector}`);
  return element;
}

function decorationElements(container: ParentNode): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(
    ".zeta-alpha-editor-decoration",
  )];
}

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
