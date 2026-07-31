import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type ZetaRendererApi } from "../../../platform/app-server/common/renderer-api.js";
import { type SyntaxAnalysisSnapshotDto, type SyntaxDiagnosticDto, type SyntaxLanguageDto, type SyntaxTokenTypeDto } from "../../../../../generated/app-server/types.js";
import { type LanguageAnalysisRequest } from "../common/languageAnalysisProviders.js";
import { LANGUAGE_DIAGNOSTIC_LANE, LANGUAGE_TOKEN_LANE, type LanguageAnalysisResult, type LanguageAnalysisWorker, type LanguageAnalysisWorkerFactory } from "../common/languageAnalysisService.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultDisposition, type LanguageWorkerResultSettler } from "../common/languageRequestCoordinator.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic, type LanguageDiagnosticResult, type LanguageToken, type LanguageTokenResult } from "../common/languageResults.js";
import { TextPosition, TextRange, type TextModelChange, type TextSnapshot } from "../common/text.js";

type SyntaxApi = ZetaRendererApi["syntax"];

const TOKEN_PRESENTATION: Readonly<Record<SyntaxTokenTypeDto, string>> = Object.freeze({
  attribute: "decorator",
  comment: "comment",
  constant: "enumMember",
  constructor: "class",
  embedded: "string",
  function: "function",
  keyword: "keyword",
  label: "label",
  module: "namespace",
  number: "number",
  operator: "operator",
  property: "property",
  string: "string",
  type: "type",
  variable: "variable",
});

let nextSyntaxDocumentId = 1;

/** Adds supported App Server syntax tokens in front of Alpha's caller-owned analysis fallback. */
export function createAppServerSyntaxAnalysisWorkerFactory(api: SyntaxApi, documentUri: string, languageId: string, fallbackFactory: LanguageAnalysisWorkerFactory): LanguageAnalysisWorkerFactory {
  assertSyntaxApi(api);
  if (typeof documentUri !== "string" || documentUri.length === 0) {
    throw new TypeError("Alpha App Server syntax analysis requires a document URI");
  }
  if (typeof fallbackFactory !== "function") {
    throw new TypeError("Alpha App Server syntax analysis requires a fallback worker factory");
  }
  const language = syntaxLanguage(languageId);
  return () => new AppServerSyntaxAnalysisWorker(
    languageId,
    language === undefined ? undefined : new SyntaxSession(api, `alpha-syntax-${nextSyntaxDocumentId++}`, documentUri, language),
    fallbackFactory(),
  );
}

class AppServerSyntaxAnalysisWorker extends DisposableOwner implements LanguageAnalysisWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler {
  private reportedBackendFailure = false;

  constructor(
    private readonly languageId: string,
    private readonly syntax: SyntaxSession | undefined,
    private readonly fallback: LanguageAnalysisWorker,
  ) {
    super();
    if (syntax) this.own(syntax);
    this.own(fallback);
  }

  async run(request: LanguageWorkerRequest<"tokens" | "diagnostics", LanguageAnalysisRequest>, signal: AbortSignal): Promise<LanguageAnalysisResult> {
    if (!this.syntax || request.payload.languageId !== this.languageId) {
      return this.fallback.run(request, signal);
    }
    if (request.lane === LANGUAGE_TOKEN_LANE) {
      try {
        const value = await this.syntax.provideTokens(request.snapshot, signal);
        this.reportedBackendFailure = false;
        return Object.freeze({ lane: LANGUAGE_TOKEN_LANE, value });
      } catch (error) {
        if (signal.aborted) throw error;
        if (!this.reportedBackendFailure) {
          this.reportedBackendFailure = true;
          console.error(`Alpha App Server ${this.languageId} syntax analysis failed; using the existing analysis fallback`, error);
        }
        return this.fallback.run(request, signal);
      }
    }
    let backend: LanguageDiagnosticResult;
    try {
      backend = await this.syntax.provideDiagnostics(request.snapshot, signal);
      this.reportedBackendFailure = false;
    } catch (error) {
      if (signal.aborted) throw error;
      if (!this.reportedBackendFailure) {
        this.reportedBackendFailure = true;
        console.error(`Alpha App Server ${this.languageId} syntax analysis failed; using the existing analysis fallback`, error);
      }
      return this.fallback.run(request, signal);
    }
    const fallback = await this.fallback.run(request, signal);
    if (fallback.lane !== LANGUAGE_DIAGNOSTIC_LANE) {
      throw new TypeError(`Alpha syntax diagnostic fallback returned '${fallback.lane}'`);
    }
    return Object.freeze({ lane: LANGUAGE_DIAGNOSTIC_LANE, value: mergeDiagnostics(backend, fallback.value) });
  }

  synchronizeModel(change: TextModelChange): void {
    this.syntax?.synchronizeModel(change);
    if (supportsModelSynchronization(this.fallback)) this.fallback.synchronizeModel(change);
  }

  settleResult(requestId: number, disposition: LanguageWorkerResultDisposition): void {
    if (supportsResultSettlement(this.fallback)) this.fallback.settleResult(requestId, disposition);
  }
}

class SyntaxSession implements IDisposable {
  private tail: Promise<void> = Promise.resolve();
  private revision: number | undefined;
  private snapshot: SyntaxAnalysisSnapshotDto | undefined;
  private disposed = false;

  constructor(
    private readonly api: SyntaxApi,
    private readonly documentId: string,
    private readonly documentUri: string,
    private readonly language: SyntaxLanguageDto,
  ) {}

  async provideTokens(snapshot: TextSnapshot, signal: AbortSignal): Promise<LanguageTokenResult> {
    return decodeTokens(await this.provideSnapshot(snapshot, signal));
  }

  async provideDiagnostics(snapshot: TextSnapshot, signal: AbortSignal): Promise<LanguageDiagnosticResult> {
    return decodeDiagnostics(await this.provideSnapshot(snapshot, signal));
  }

  private async provideSnapshot(snapshot: TextSnapshot, signal: AbortSignal): Promise<SyntaxAnalysisSnapshotDto> {
    this.ensureAlive();
    signal.throwIfAborted();
    const result = await this.enqueue(async () => {
      signal.throwIfAborted();
      if (this.revision !== snapshot.version || !this.snapshot) {
        const opened = await this.api.open({
          documentId: this.documentId,
          documentUri: this.documentUri,
          language: this.language,
          revision: snapshot.version,
          text: snapshot.getText(),
        });
        this.acceptSnapshot(opened, snapshot.version);
      }
      return this.snapshot!;
    });
    signal.throwIfAborted();
    return result;
  }

  synchronizeModel(change: TextModelChange): void {
    this.ensureAlive();
    const previousRevision = change.version - 1;
    const edits = change.changes.map(item => Object.freeze({
      startUtf16: item.rangeOffset,
      endUtf16: item.rangeOffset + item.rangeLength,
      text: item.text,
    }));
    void this.enqueue(async () => {
      if (this.revision !== previousRevision) return;
      try {
        const changed = await this.api.change({
          documentId: this.documentId,
          previousRevision,
          revision: change.version,
          edits,
        });
        this.acceptSnapshot(changed, change.version);
      } catch {
        this.revision = undefined;
        this.snapshot = undefined;
      }
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    void this.enqueue(async () => {
      try {
        await this.api.close({ documentId: this.documentId });
      } catch {
        // Connection teardown also releases every connection-owned syntax document.
      }
      this.revision = undefined;
      this.snapshot = undefined;
    });
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private acceptSnapshot(snapshot: SyntaxAnalysisSnapshotDto, expectedRevision: number): void {
    validateSnapshot(snapshot, expectedRevision);
    this.revision = snapshot.revision;
    this.snapshot = snapshot;
  }

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.tail.then(operation);
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("Alpha App Server syntax session is already disposed");
  }
}

function decodeTokens(snapshot: SyntaxAnalysisSnapshotDto): LanguageTokenResult {
  const tokens: LanguageToken[] = [];
  let lineIndex = 0;
  let startColumn = 0;
  for (let index = 0; index < snapshot.tokens.data.length; index += 5) {
    const deltaLine = snapshot.tokens.data[index]!;
    const deltaStart = snapshot.tokens.data[index + 1]!;
    const length = snapshot.tokens.data[index + 2]!;
    const tokenTypeIndex = snapshot.tokens.data[index + 3]!;
    const modifierBits = snapshot.tokens.data[index + 4]!;
    lineIndex += deltaLine;
    startColumn = deltaLine === 0 ? startColumn + deltaStart : deltaStart;
    const syntaxTokenType = snapshot.tokens.legend[tokenTypeIndex];
    const tokenType = syntaxTokenType === undefined ? undefined : TOKEN_PRESENTATION[syntaxTokenType];
    if (!tokenType || length === 0 || modifierBits !== 0) {
      throw new TypeError("App Server returned an unsupported Alpha syntax token");
    }
    tokens.push(Object.freeze({
      range: TextRange.from(TextPosition.at(lineIndex, startColumn), TextPosition.at(lineIndex, startColumn + length)),
      tokenType,
      modifiers: Object.freeze([]),
    }));
  }
  return Object.freeze({ tokens: Object.freeze(tokens) });
}

function decodeDiagnostics(snapshot: SyntaxAnalysisSnapshotDto): LanguageDiagnosticResult {
  const diagnostics = snapshot.diagnostics.map(decodeDiagnostic);
  return Object.freeze({ diagnostics: Object.freeze(diagnostics) });
}

function decodeDiagnostic(diagnostic: SyntaxDiagnosticDto): LanguageDiagnostic {
  if (diagnostic.severity !== "error") {
    throw new TypeError(`App Server returned unsupported syntax diagnostic severity '${diagnostic.severity}'`);
  }
  return Object.freeze({
    range: TextRange.from(
      TextPosition.at(diagnostic.range.start.line, diagnostic.range.start.character),
      TextPosition.at(diagnostic.range.end.line, diagnostic.range.end.character),
    ),
    severity: LanguageDiagnosticSeverity.Error,
    message: diagnostic.message,
    source: diagnostic.source,
  });
}

function mergeDiagnostics(backend: LanguageDiagnosticResult, fallback: LanguageDiagnosticResult): LanguageDiagnosticResult {
  const diagnostics: LanguageDiagnostic[] = [];
  const seen = new Set<string>();
  for (const diagnostic of [...backend.diagnostics, ...fallback.diagnostics]) {
    const key = `${diagnostic.range.start.lineIndex}:${diagnostic.range.start.columnIndex}:${diagnostic.range.end.lineIndex}:${diagnostic.range.end.columnIndex}:${diagnostic.severity}:${diagnostic.message}:${diagnostic.source ?? ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    diagnostics.push(diagnostic);
  }
  return Object.freeze({ diagnostics: Object.freeze(diagnostics) });
}

function validateSnapshot(snapshot: SyntaxAnalysisSnapshotDto, expectedRevision: number): void {
  if (!snapshot || snapshot.revision !== expectedRevision || typeof snapshot.resultId !== "string" || snapshot.tokens.data.length % 5 !== 0) {
    throw new TypeError("App Server returned an invalid Alpha syntax-token snapshot");
  }
  if (
    !Array.isArray(snapshot.tokens.legend) ||
    new Set(snapshot.tokens.legend).size !== snapshot.tokens.legend.length ||
    snapshot.tokens.legend.some(tokenType => TOKEN_PRESENTATION[tokenType] === undefined)
  ) {
    throw new TypeError("App Server returned an invalid Alpha syntax-token legend");
  }
  if (snapshot.tokens.data.some(value => !Number.isInteger(value) || value < 0 || value > 0xffff_ffff)) {
    throw new TypeError("App Server returned invalid Alpha syntax-token data");
  }
}

function assertSyntaxApi(api: SyntaxApi): void {
  if (!api || typeof api.open !== "function" || typeof api.change !== "function" || typeof api.close !== "function") {
    throw new TypeError("Alpha App Server syntax analysis requires the syntax renderer API");
  }
}

function syntaxLanguage(languageId: string): SyntaxLanguageDto | undefined {
  switch (languageId) {
    case "json": return "json";
    case "jsonc": return "jsonc";
    case "rust": return "rust";
    default: return undefined;
  }
}

function supportsModelSynchronization(worker: LanguageAnalysisWorker): worker is LanguageAnalysisWorker & LanguageWorkerModelSynchronizer {
  return typeof (worker as Partial<LanguageWorkerModelSynchronizer>).synchronizeModel === "function";
}

function supportsResultSettlement(worker: LanguageAnalysisWorker): worker is LanguageAnalysisWorker & LanguageWorkerResultSettler {
  return typeof (worker as Partial<LanguageWorkerResultSettler>).settleResult === "function";
}
