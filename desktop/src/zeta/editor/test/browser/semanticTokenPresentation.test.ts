import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { SemanticTokenModifier, SemanticTokenPresentation, createAlphaSemanticTokenSource, projectAlphaSemanticTokenLine, resolveAlphaSemanticTokenModifiers, resolveAlphaSemanticTokenPresentation, type ResolvedSemanticToken } from "../../browser/view/semanticTokenPresentation.js";
import { LanguageResultAcceptance } from "../../common/languages/languageResultStore.js";
import { LanguageTokenLineIndex } from "../../common/tokens/languageTokenLineIndex.js";
import { createLanguageTokenStore, type LanguageToken } from "../../common/languages/languageResults.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";

test("Default resolver maps only Alpha's explicit semantic vocabulary", () => {
  assert.equal(resolveAlphaSemanticTokenPresentation(token(0, 0, 1, "keyword")), SemanticTokenPresentation.Keyword);
  assert.equal(resolveAlphaSemanticTokenPresentation(token(0, 0, 1, "method")), SemanticTokenPresentation.Function);
  assert.equal(resolveAlphaSemanticTokenPresentation(token(0, 0, 1, "plugin-controlled-class")), undefined);
});

test("Semantic token modifiers use Alpha's closed presentation vocabulary", () => {
  assert.deepEqual(
    resolveAlphaSemanticTokenModifiers(token(0, 0, 1, "variable", ["declaration", "readonly", "unknown-plugin-modifier", "definition"])),
    [SemanticTokenModifier.Declaration, SemanticTokenModifier.Readonly],
  );

  const dom = new JSDOM("<!doctype html><body><code></code></body>");
  const element = requiredElement<HTMLElement>(dom.window.document, "code");
  projectAlphaSemanticTokenLine(element, "name", [presented(
    0,
    4,
    SemanticTokenPresentation.Variable,
    [SemanticTokenModifier.Declaration, SemanticTokenModifier.Readonly],
  )]);
  const rendered = requiredElement<HTMLElement>(element, ".zeta-alpha-editor-token");
  assert.equal(rendered.classList.contains(SemanticTokenModifier.Declaration), true);
  assert.equal(rendered.classList.contains(SemanticTokenModifier.Readonly), true);
  assert.equal(rendered.textContent, "name");
  dom.window.close();
});

test("Semantic token source resolves immutable named lines without owning common state", () => {
  using model = new TextModel("const value");
  using store = createLanguageTokenStore(model);
  assert.equal(store.accept({
    requestId: 1,
    textModel: model,
    modelVersion: model.version,
    value: {
      tokens: [
        token(0, 0, 5, "keyword"),
        token(0, 6, 11, "plugin-variable"),
      ],
    },
  }), LanguageResultAcceptance.Applied);
  using index = new LanguageTokenLineIndex(store);
  const source = createAlphaSemanticTokenSource(index, entry => (
    entry.tokenType === "plugin-variable"
      ? SemanticTokenPresentation.Variable
      : resolveAlphaSemanticTokenPresentation(entry)
  ));

  assert.equal(source.textModel, model);
  assert.deepEqual(source.lines, [{
    lineIndex: 0,
    tokens: [{
      startColumn: 0,
      endColumn: 5,
      presentation: SemanticTokenPresentation.Keyword,
    }, {
      startColumn: 6,
      endColumn: 11,
      presentation: SemanticTokenPresentation.Variable,
    }],
  }]);

  index.dispose();
  assert.throws(() => source.lines, /already disposed/);
  assert.equal(store.result!.value.tokens.length, 2);
});

test("Semantic line projection is HTML-safe and preserves exact text", () => {
  const dom = new JSDOM("<!doctype html><body><code>old</code></body>");
  const element = requiredElement<HTMLElement>(dom.window.document, "code");
  const lineText = "const <tag> = 42";
  projectAlphaSemanticTokenLine(element, lineText, [
    presented(0, 5, SemanticTokenPresentation.Keyword),
    presented(6, 11, SemanticTokenPresentation.Variable),
    presented(14, 16, SemanticTokenPresentation.Number),
  ]);

  assert.equal(element.textContent, lineText);
  assert.equal(element.querySelector("tag"), null);
  assert.deepEqual([...element.querySelectorAll(".zeta-alpha-editor-token")].map(tokenElement => ({
    className: tokenElement.className,
    text: tokenElement.textContent,
  })), [{
    className: "zeta-alpha-editor-token token-keyword",
    text: "const",
  }, {
    className: "zeta-alpha-editor-token token-variable",
    text: "<tag>",
  }, {
    className: "zeta-alpha-editor-token token-number",
    text: "42",
  }]);
  dom.window.close();
});

test("Semantic line projection composes lexical bracket colors without changing token text", () => {
  const dom = new JSDOM("<!doctype html><body><code></code></body>");
  const element = requiredElement<HTMLElement>(dom.window.document, "code");
  projectAlphaSemanticTokenLine(element, "fn(a)", [presented(0, 2, SemanticTokenPresentation.Function)], [
    { startColumn: 2, endColumn: 3, level: 1 },
    { startColumn: 4, endColumn: 5, level: 1 },
  ]);
  assert.equal(element.textContent, "fn(a)");
  assert.deepEqual([...element.querySelectorAll(".zeta-alpha-editor-bracket-level-1")].map(entry => entry.textContent), ["(", ")"]);
  dom.window.close();
});

test("Invalid semantic line input fails before replacing existing DOM", () => {
  const dom = new JSDOM("<!doctype html><body><code><b>stable</b></code></body>");
  const element = requiredElement<HTMLElement>(dom.window.document, "code");
  const existing = element.firstElementChild;

  assert.throws(() => projectAlphaSemanticTokenLine(element, "abcd", [
    presented(0, 3, SemanticTokenPresentation.Keyword),
    presented(2, 4, SemanticTokenPresentation.String),
  ]), /sorted, non-overlapping/);
  assert.equal(element.firstElementChild, existing);
  assert.equal(element.innerHTML, "<b>stable</b>");

  assert.throws(() => projectAlphaSemanticTokenLine(element, "abcd", [
    presented(0, 1, "worker-css" as SemanticTokenPresentation),
  ]), /Unknown Alpha semantic token presentation/);
  assert.equal(element.innerHTML, "<b>stable</b>");
  dom.window.close();
});

function token(lineIndex: number, startColumn: number, endColumn: number, tokenType: string, modifiers: readonly string[] = []): LanguageToken {
  return {
    range: TextRange.from(
      TextPosition.at(lineIndex, startColumn),
      TextPosition.at(lineIndex, endColumn),
    ),
    tokenType,
    modifiers,
  };
}

function presented(startColumn: number, endColumn: number, presentation: SemanticTokenPresentation, modifiers?: readonly SemanticTokenModifier[]): ResolvedSemanticToken {
  return { startColumn, endColumn, presentation, ...(modifiers ? { modifiers } : {}) };
}

function requiredElement<T extends Element>(root: ParentNode, selector: string): T {
  const element = root.querySelector<T>(selector);
  assert.ok(element);
  return element;
}
