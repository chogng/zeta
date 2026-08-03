import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { type AlphaTextMeasurer } from "../../browser/fontMetrics.js";
import { AlphaSemanticTokenPresentation, createAlphaSemanticTokenSource } from "../../browser/semanticTokenPresentation.js";
import { LanguageResultAcceptance } from "../../common/languageResultStore.js";
import { LanguageTokenLineIndex } from "../../common/languageTokenLineIndex.js";
import { createLanguageTokenStore, type LanguageToken } from "../../common/languageResults.js";
import { TextPosition, TextRange } from "../../common/text.js";
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
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { AlphaEditorViewport } = await import("../../browser/alphaEditorViewport.js");
const { AlphaEditorLineWrapping } = await import("../../browser/visualLineProjection.js");

test("Viewport projects tokens only for virtualized lines and preserves overlapping rows", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel(lines(12).join("\n"));
  using store = createLanguageTokenStore(model);
  acceptTokens(store, model, 1, [
    token(0, 0, 4, "keyword"),
    token(3, 0, 4, "string"),
    token(10, 0, 4, "number"),
  ]);
  using index = new LanguageTokenLineIndex(store);
  const source = createAlphaSemanticTokenSource(index);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    overscanLineCount: 0,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: source,
  });
  viewport.layout({ width: 200, height: 40 });

  assert.deepEqual(renderedTokenLines(viewport.element), ["0"]);
  viewport.scrollTo({ left: 0, top: 40 });
  const line3 = requiredLine(viewport.element, 3);
  const line3Token = requiredElement(line3, ".zeta-alpha-editor-token");
  assert.deepEqual(renderedTokenLines(viewport.element), ["3"]);

  viewport.scrollTo({ left: 0, top: 60 });
  assert.equal(requiredLine(viewport.element, 3), line3);
  assert.equal(requiredElement(line3, ".zeta-alpha-editor-token"), line3Token);
  assert.equal(viewport.element.querySelector('[data-line-index="10"]'), null);
  dom.window.close();
});

test("Same-version token replacement rerenders visible text and model edits clear stale spans", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel("<tag> value");
  using store = createLanguageTokenStore(model);
  acceptTokens(store, model, 1, [token(0, 0, 5, "string")]);
  using index = new LanguageTokenLineIndex(store);
  const source = createAlphaSemanticTokenSource(index);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: source,
  });
  viewport.layout({ width: 200, height: 20 });
  const textElement = requiredElement<HTMLElement>(requiredLine(viewport.element, 0), ".zeta-alpha-editor-line-text");
  assert.equal(textElement.textContent, "<tag> value");
  assert.equal(textElement.querySelector("tag"), null);
  assert.equal(requiredElement(textElement, ".zeta-alpha-editor-token").classList.contains(AlphaSemanticTokenPresentation.String), true);

  acceptTokens(store, model, 2, [token(0, 6, 11, "variable")]);
  assert.equal(textElement.textContent, "<tag> value");
  assert.deepEqual([...textElement.querySelectorAll(".zeta-alpha-editor-token")].map(element => ({
    className: element.className,
    text: element.textContent,
  })), [{
    className: "zeta-alpha-editor-token token-variable",
    text: "value",
  }]);

  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "X",
  }]);
  assert.equal(store.result, undefined);
  assert.equal(textElement.textContent, "X<tag> value");
  assert.equal(textElement.querySelector(".zeta-alpha-editor-token"), null);
  dom.window.close();
});

test("Viewport clips semantic token spans to every soft-wrapped text fragment", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel("abcdef");
  using store = createLanguageTokenStore(model);
  acceptTokens(store, model, 1, [token(0, 1, 5, "keyword")]);
  using index = new LanguageTokenLineIndex(store);
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: createAlphaSemanticTokenSource(index),
    lineWrapping: AlphaEditorLineWrapping.On,
  });
  viewport.layout({ width: 70, height: 60 });

  assert.deepEqual(lineTokenFragments(viewport.element), [{
    lineIndex: "0",
    text: "b",
  }, {
    lineIndex: "1",
    text: "cd",
  }, {
    lineIndex: "2",
    text: "e",
  }]);

  dom.window.close();
});

test("Viewport rejects cross-model token sources and owns none of their common state", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel("alpha");
  using otherModel = new TextModel("other");
  using store = createLanguageTokenStore(model);
  using otherStore = createLanguageTokenStore(otherModel);
  using index = new LanguageTokenLineIndex(store);
  using otherIndex = new LanguageTokenLineIndex(otherStore);
  const source = createAlphaSemanticTokenSource(index);
  const otherSource = createAlphaSemanticTokenSource(otherIndex);

  assert.throws(() => new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: otherSource,
  }), /must share one text model/);
  const viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: source,
  });
  viewport.dispose();

  acceptTokens(store, model, 1, [token(0, 0, 5, "keyword")]);
  assert.equal(index.getLineTokens(0).length, 1);
  model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 5)),
    text: "!",
  }]);
  assert.equal(model.getText(), "alpha!");
  dom.window.close();
});

test("Viewport resolves semantic tokens only for virtualized lines", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const container = requiredElement<HTMLElement>(dom.window.document, "main");
  using model = new TextModel(lines(1_000).join("\n"));
  using store = createLanguageTokenStore(model);
  acceptTokens(store, model, 1, Array.from({ length: 1_000 }, (_, lineIndex) => token(lineIndex, 0, 4, "keyword")));
  using index = new LanguageTokenLineIndex(store);
  let resolverCalls = 0;
  const source = createAlphaSemanticTokenSource(index, () => {
    resolverCalls += 1;
    return AlphaSemanticTokenPresentation.Keyword;
  });
  using viewport = new AlphaEditorViewport({
    container,
    model,
    lineHeight: 20,
    overscanLineCount: 0,
    textMeasurer: new FixedTextMeasurer(),
    semanticTokenSource: source,
  });

  viewport.layout({ width: 200, height: 20 });
  assert.equal(resolverCalls, 1);
  viewport.scrollTo({ left: 0, top: 500 * 20 });
  assert.equal(resolverCalls, 2);
  acceptTokens(store, model, 2, Array.from({ length: 1_000 }, (_, lineIndex) => token(lineIndex, 0, 4, "keyword")));
  assert.equal(resolverCalls, 3);
  dom.window.close();
});

function acceptTokens(
  store: ReturnType<typeof createLanguageTokenStore>,
  model: TextModel,
  requestId: number,
  tokens: readonly LanguageToken[],
): void {
  assert.equal(store.accept({
    requestId,
    textModel: model,
    modelVersion: model.version,
    value: { tokens },
  }), LanguageResultAcceptance.Applied);
}

function token(lineIndex: number, startColumn: number, endColumn: number, tokenType: string): LanguageToken {
  return {
    range: TextRange.from(
      TextPosition.at(lineIndex, startColumn),
      TextPosition.at(lineIndex, endColumn),
    ),
    tokenType,
    modifiers: [],
  };
}

function lines(count: number): string[] {
  return Array.from({ length: count }, (_, index) => `line${index}`);
}

function renderedTokenLines(root: ParentNode): string[] {
  return [...root.querySelectorAll(".zeta-alpha-editor-token")].map(element => (
    (element.parentElement?.parentElement as HTMLElement).dataset.lineIndex!
  ));
}

function lineTokenFragments(root: ParentNode): { readonly lineIndex: string | undefined; readonly text: string | null }[] {
  return [...root.querySelectorAll<HTMLElement>(".zeta-alpha-editor-token")].map(element => ({
    lineIndex: element.parentElement?.parentElement?.dataset.lineIndex,
    text: element.textContent,
  }));
}

function requiredLine(root: ParentNode, lineIndex: number): HTMLElement {
  return requiredElement(root, `[data-line-index="${lineIndex}"]`);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
  const element = root.querySelector<T>(selector);
  assert.ok(element);
  return element;
}

class FixedTextMeasurer implements AlphaTextMeasurer {
  readonly horizontalPadding = 24;
  readonly contentLeftPadding = 12;

  refresh(): boolean {
    return false;
  }

  measureLineWidth(text: string): number {
    return text.length * 10;
  }
}
