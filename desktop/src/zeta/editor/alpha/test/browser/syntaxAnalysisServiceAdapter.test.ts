import assert from "node:assert/strict";
import test from "node:test";
import { AppServerSyntaxAnalysisService, type ISyntaxAnalysisApi } from "../../../../platform/syntax/browser/appServerSyntaxAnalysisService.js";
import { createSyntaxAnalysisServiceAdapter } from "../../browser/syntaxAnalysisServiceAdapter.js";
import { LANGUAGE_DIAGNOSTIC_LANE, LANGUAGE_TOKEN_LANE, type LanguageAnalysisResult, type LanguageAnalysisWorker } from "../../common/languageAnalysisService.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest } from "../../common/languageRequestCoordinator.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnosticResult } from "../../common/languageResults.js";
import { TextModelChangeReason, TextPosition, TextRange, type TextModelChange, type TextSnapshot } from "../../common/text.js";
import { type LanguageAnalysisRequest } from "../../common/languageAnalysisProviders.js";
import { type SyntaxAnalysisSnapshotDto, type SyntaxChangeParams, type SyntaxCloseParams, type SyntaxOpenParams } from "../../../../../../generated/app-server/types.js";

test("App Server syntax adapter forwards the canonical service lifecycle", async () => {
  const api = new RecordingSyntaxApi();
  const service = new AppServerSyntaxAnalysisService(api);

  await service.open({ documentId: "document-1", documentUri: "file:///workspace/main.rs", language: "rust", revision: 1, text: "fn main() {}" });
  await service.change({ documentId: "document-1", previousRevision: 1, revision: 2, edits: [{ startUtf16: 3, endUtf16: 7, text: "entry" }] });
  await service.close({ documentId: "document-1" });

  assert.equal(api.openCalls[0]?.documentUri, "file:///workspace/main.rs");
  assert.deepEqual(api.changeCalls[0]?.edits, [{ startUtf16: 3, endUtf16: 7, text: "entry" }]);
  assert.deepEqual(api.closeCalls, [{ documentId: "document-1" }]);
});

test("Alpha syntax service adapter decodes Rust tokens and synchronizes model transactions", async () => {
  const api = new RecordingSyntaxApi();
  const fallback = new RecordingFallbackWorker();
  const factory = createSyntaxAnalysisServiceAdapter(new AppServerSyntaxAnalysisService(api), "file:///workspace/main.rs", "rust", () => fallback);
  using worker = factory();

  const first = await worker.run(request(1, LANGUAGE_TOKEN_LANE, "rust", snapshot(1, "fn main() {\n  x\n}")), new AbortController().signal);
  assert.equal(first.lane, LANGUAGE_TOKEN_LANE);
  assert.deepEqual(first.value.tokens.map(token => ({
    start: [token.range.start.lineIndex, token.range.start.columnIndex],
    end: [token.range.end.lineIndex, token.range.end.columnIndex],
    type: token.tokenType,
  })), [
    { start: [0, 0], end: [0, 2], type: "keyword" },
    { start: [0, 3], end: [0, 7], type: "function" },
    { start: [1, 2], end: [1, 3], type: "variable" },
  ]);
  assert.deepEqual(api.openCalls, [{
    documentId: "alpha-syntax-1",
    documentUri: "file:///workspace/main.rs",
    language: "rust",
    revision: 1,
    text: "fn main() {\n  x\n}",
  }]);

  (worker as LanguageAnalysisWorker & LanguageWorkerModelSynchronizer).synchronizeModel(change(2, 14, 1, "value"));
  await worker.run(request(2, LANGUAGE_TOKEN_LANE, "rust", snapshot(2, "fn main() {\n  value\n}")), new AbortController().signal);
  assert.deepEqual(api.changeCalls, [{
    documentId: "alpha-syntax-1",
    previousRevision: 1,
    revision: 2,
    edits: [{ startUtf16: 14, endUtf16: 15, text: "value" }],
  }]);

  const diagnostics = await worker.run(request(3, LANGUAGE_DIAGNOSTIC_LANE, "rust", snapshot(2, "fn main() {\n  value\n}")), new AbortController().signal);
  const typescript = await worker.run(request(4, LANGUAGE_TOKEN_LANE, "typescript", snapshot(2, "fn main() {\n  value\n}")), new AbortController().signal);
  assert.equal(diagnostics.lane, LANGUAGE_DIAGNOSTIC_LANE);
  assert.equal(typescript.lane, LANGUAGE_TOKEN_LANE);
  assert.deepEqual(fallback.calls, ["diagnostics:rust", "tokens:typescript"]);
});

test("Alpha syntax service adapter routes JSON and JSONC through the backend", async () => {
  for (const languageId of ["json", "jsonc"] as const) {
    const api = new RecordingSyntaxApi();
    using worker = createSyntaxAnalysisServiceAdapter(new AppServerSyntaxAnalysisService(api), `file:///workspace/settings.${languageId}`, languageId, () => new RecordingFallbackWorker())();

    await worker.run(request(1, LANGUAGE_TOKEN_LANE, languageId, snapshot(1, "{\"enabled\":true}")), new AbortController().signal);

    assert.equal(api.openCalls[0]?.language, languageId);
  }
});

test("Alpha syntax service adapter uses the existing fallback when backend analysis fails", async context => {
  context.mock.method(console, "error", () => undefined);
  const api = new RecordingSyntaxApi();
  api.openError = new Error("backend unavailable");
  const fallback = new RecordingFallbackWorker();
  using worker = createSyntaxAnalysisServiceAdapter(new AppServerSyntaxAnalysisService(api), "file:///workspace/main.rs", "rust", () => fallback)();

  const result = await worker.run(request(1, LANGUAGE_TOKEN_LANE, "rust", snapshot(1, "fn main() {}")), new AbortController().signal);
  assert.equal(result.lane, LANGUAGE_TOKEN_LANE);
  assert.deepEqual(fallback.calls, ["tokens:rust"]);
});

test("Alpha syntax service adapter merges backend and fallback diagnostics", async () => {
  const api = new RecordingSyntaxApi();
  api.diagnostics = [{
    range: { start: { line: 0, character: 3 }, end: { line: 0, character: 3 } },
    severity: "error",
    message: "Missing syntax",
    source: "tree-sitter",
  }];
  const fallback = new RecordingFallbackWorker();
  fallback.diagnostics = [{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 1)),
    severity: LanguageDiagnosticSeverity.Warning,
    message: "Fallback diagnostic",
    source: "alpha.lexical",
  }];
  using worker = createSyntaxAnalysisServiceAdapter(new AppServerSyntaxAnalysisService(api), "file:///workspace/main.rs", "rust", () => fallback)();

  const result = await worker.run(request(1, LANGUAGE_DIAGNOSTIC_LANE, "rust", snapshot(1, "fn (")), new AbortController().signal);

  assert.equal(result.lane, LANGUAGE_DIAGNOSTIC_LANE);
  assert.deepEqual(result.value.diagnostics.map(diagnostic => [diagnostic.message, diagnostic.source]), [
    ["Missing syntax", "tree-sitter"],
    ["Fallback diagnostic", "alpha.lexical"],
  ]);
});

class RecordingSyntaxApi implements ISyntaxAnalysisApi {
  readonly openCalls: SyntaxOpenParams[] = [];
  readonly changeCalls: SyntaxChangeParams[] = [];
  readonly closeCalls: SyntaxCloseParams[] = [];
  openError: Error | undefined;
  diagnostics: SyntaxAnalysisSnapshotDto["diagnostics"] = [];

  async open(params: SyntaxOpenParams): Promise<SyntaxAnalysisSnapshotDto> {
    this.openCalls.push(params);
    if (this.openError) throw this.openError;
    return analysisSnapshot(params.revision, this.diagnostics);
  }

  async change(params: SyntaxChangeParams): Promise<SyntaxAnalysisSnapshotDto> {
    this.changeCalls.push(params);
    return analysisSnapshot(params.revision, this.diagnostics);
  }

  async close(params: SyntaxCloseParams): Promise<void> {
    this.closeCalls.push(params);
  }
}

class RecordingFallbackWorker implements LanguageAnalysisWorker {
  readonly calls: string[] = [];
  diagnostics: LanguageDiagnosticResult["diagnostics"] = [];

  run(request: LanguageWorkerRequest<"tokens" | "diagnostics", LanguageAnalysisRequest>): Promise<LanguageAnalysisResult> {
    this.calls.push(`${request.lane}:${request.payload.languageId}`);
    return Promise.resolve(request.lane === LANGUAGE_TOKEN_LANE ? {
      lane: LANGUAGE_TOKEN_LANE,
      value: { tokens: [] },
    } : {
      lane: LANGUAGE_DIAGNOSTIC_LANE,
      value: { diagnostics: this.diagnostics },
    });
  }

  dispose(): void {}

  [Symbol.dispose](): void {
    this.dispose();
  }
}

function request(requestId: number, lane: "tokens" | "diagnostics", languageId: string, value: TextSnapshot): LanguageWorkerRequest<"tokens" | "diagnostics", LanguageAnalysisRequest> {
  return Object.freeze({ requestId, lane, snapshot: value, payload: Object.freeze({ languageId }) });
}

function snapshot(version: number, text: string): TextSnapshot {
  return Object.freeze({
    version,
    length: text.length,
    lineCount: text.split("\n").length,
    getText: () => text,
    getTextBetweenOffsets: (startOffset: number, endOffset: number) => text.slice(startOffset, endOffset),
  });
}

function change(version: number, rangeOffset: number, rangeLength: number, text: string): TextModelChange {
  return Object.freeze({
    version,
    transactionId: version,
    reason: TextModelChangeReason.Edit,
    changes: Object.freeze([Object.freeze({
      range: TextRange.from(TextPosition.at(1, 2), TextPosition.at(1, 3)),
      rangeOffset,
      rangeLength,
      text,
    })]),
  });
}

function analysisSnapshot(revision: number, diagnostics: SyntaxAnalysisSnapshotDto["diagnostics"]): SyntaxAnalysisSnapshotDto {
  return {
    revision,
    resultId: String(revision),
    hasErrors: diagnostics.length > 0,
    tokens: {
      legend: ["attribute", "comment", "constant", "constructor", "embedded", "function", "keyword", "label", "module", "number", "operator", "property", "string", "type", "variable"],
      data: [0, 0, 2, 6, 0, 0, 3, 4, 5, 0, 1, 2, 1, 14, 0],
    },
    foldingRanges: [],
    symbols: [],
    diagnostics,
  };
}
