import { strict as assert } from "node:assert";
import test from "node:test";
import { DocumentSymbolService, type LanguageDocumentSymbolProvider } from "../../contrib/documentSymbols/common/documentSymbols.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { SyntaxProviderRegistry } from "../../common/languages/syntax/syntaxProviders.js";
import { SyntaxService, type SyntaxWorker } from "../../common/languages/syntax/syntaxService.js";
import { LanguageFeatureProviderRegistry } from "../../common/languages/languageFeatureRegistry.js";
import { RustSyntaxWorker, RustSyntaxDocumentSymbolProvider, RustSyntaxFactsService } from "../../browser/services/rustSyntaxFactsService.js";

test("Rust syntax facts feed Aster token, diagnostic, and document-symbol services from one revision request", async () => {
  using model = new TextModel("fn main() {\n  /* hi\n  */\n}\n");
  let calls = 0;
  using facts = new RustSyntaxFactsService({
    analyze: async params => {
      calls += 1;
      return {
        revision: params.revision,
        hasErrors: true,
        tokens: [
          { kind: "variable", range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 7 } } },
          { kind: "keyword", range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } } },
          { kind: "function", range: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } } },
          { kind: "comment", range: { start: { lineIndex: 1, columnIndex: 2 }, end: { lineIndex: 2, columnIndex: 4 } } },
        ],
        foldingRanges: [],
        symbols: [{
          name: "main",
          kind: "function",
          range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 3, columnIndex: 1 } },
          selectionRange: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } },
        }],
        diagnostics: [{
          kind: "missing",
          range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } },
        }],
      };
    },
  });
  using syntaxProviders = new SyntaxProviderRegistry();
  using syntax = new SyntaxService(model, syntaxProviders, {
    workerDecorator: fallback => new RustSyntaxWorker(facts, fallback),
  });
  using symbolProviders = new LanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>();
  using symbols = new DocumentSymbolService(model, symbolProviders, {
    fallbackProviders: [new RustSyntaxDocumentSymbolProvider(facts)],
  });

  await syntax.requestAll("rust");
  const documentSymbols = await symbols.provideDocumentSymbols("rust");

  assert.equal(calls, 1);
  assert.deepEqual(syntax.tokens.result!.value.tokens.map(token => [
    token.range.start.lineIndex,
    token.range.start.columnIndex,
    token.range.end.lineIndex,
    token.range.end.columnIndex,
    token.tokenType,
  ]), [
    [0, 0, 0, 2, "keyword"],
    [0, 2, 0, 3, "variable"],
    [0, 3, 0, 7, "function"],
    [1, 2, 1, 7, "comment"],
    [2, 0, 2, 4, "comment"],
  ]);
  assert.deepEqual(syntax.diagnostics.result!.value.diagnostics.map(diagnostic => [diagnostic.code, diagnostic.message, diagnostic.source]), [
    ["syntax-missing", "Missing required syntax", "zeta-syntax"],
  ]);
  assert.deepEqual(documentSymbols.map(symbol => [symbol.name, symbol.kind, symbol.selectionRange.start.lineIndex, symbol.selectionRange.start.columnIndex]), [
    ["main", "function", 0, 3],
  ]);
});

test("Rust syntax facts leave unsupported languages and oversized documents to Aster's fallback worker", async () => {
  using model = new TextModel("const value = 1;");
  let fallbackCalls = 0;
  let syntaxCalls = 0;
  using facts = new RustSyntaxFactsService({
    analyze: async () => {
      syntaxCalls += 1;
      throw new Error("Unsupported languages and oversized documents must not call the syntax endpoint");
    },
  });
  const fallback: SyntaxWorker = {
    run: async request => {
      fallbackCalls += 1;
      return request.lane === "tokens"
        ? { lane: "tokens" as const, value: { tokens: [] } }
        : { lane: "diagnostics" as const, value: { diagnostics: [] } };
    },
    dispose() {},
    [Symbol.dispose]() {},
  };
  using worker = new RustSyntaxWorker(facts, fallback);
  const result = await worker.run({
    requestId: 1,
    lane: "tokens",
    payload: { languageId: "markdown" },
    snapshot: model.createSnapshot(),
  }, new AbortController().signal);

  assert.equal(fallbackCalls, 1);
  assert.equal(syntaxCalls, 0);
  assert.equal(result.lane, "tokens");

  const oversized = "x".repeat(4 * 1024 * 1024 + 1);
  const snapshot = Object.freeze({
    version: 1,
    length: oversized.length,
    lineCount: 1,
    getText: () => oversized,
    getTextBetweenOffsets: (startOffset: number, endOffset: number) => oversized.slice(startOffset, endOffset),
  });
  assert.equal(await facts.analyze("rust", snapshot, new AbortController().signal), undefined);
  assert.equal(syntaxCalls, 0);
});

test("Rust syntax symbols remain a fallback behind registered Aster providers", async () => {
  using model = new TextModel("fn main() {}");
  let syntaxCalls = 0;
  using facts = new RustSyntaxFactsService({
    analyze: async () => {
      syntaxCalls += 1;
      throw new Error("Registered symbol providers must take precedence");
    },
  });
  using providers = new LanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>();
  using registration = providers.register({
    languageIds: ["rust"],
    provideDocumentSymbols: () => [{
      name: "extensionSymbol",
      kind: "function",
      range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
      selectionRange: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 2)),
    }],
  });
  using symbols = new DocumentSymbolService(model, providers, {
    fallbackProviders: [new RustSyntaxDocumentSymbolProvider(facts)],
  });

  assert.deepEqual((await symbols.provideDocumentSymbols("rust")).map(symbol => symbol.name), ["extensionSymbol"]);
  assert.equal(syntaxCalls, 0);
});
