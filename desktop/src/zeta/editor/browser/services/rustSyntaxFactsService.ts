import { raceCancellation } from "../../../base/common/cancellation.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { ISyntaxApi, SyntaxAnalyzeResult, SyntaxDiagnostic, SyntaxSymbol, SyntaxToken } from "../../../platform/syntax/common/syntaxApi.js";
import { type LanguageDocumentSymbol, type LanguageDocumentSymbolProvider } from "../../contrib/documentSymbols/common/documentSymbols.js";
import { TextPosition, TextRange, type TextSnapshot } from "../../common/core/text.js";
import type { SyntaxRequest } from "../../common/languages/syntax/syntaxProviders.js";
import { SYNTAX_DIAGNOSTIC_LANE, SYNTAX_TOKEN_LANE, type SyntaxResult, type SyntaxWorker } from "../../common/languages/syntax/syntaxService.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnosticResult, type LanguageToken, type LanguageTokenResult } from "../../common/languages/languageResults.js";
import { type LanguageWorkerModelSynchronizer, type LanguageWorkerRequest, type LanguageWorkerResultDisposition, type LanguageWorkerResultSettler } from "../../common/languages/languageRequestCoordinator.js";

const MAX_SYNTAX_INPUT_BYTES = 4 * 1024 * 1024;

interface CachedSyntaxFacts {
  readonly key: string;
  readonly promise: Promise<SyntaxAnalyzeResult>;
}

/**
 * Shares one revision-bound Rust syntax request between Aster consumers.
 *
 * This browser adapter owns no editor state: callers retain their own result stores and use the
 * projected facts only while the captured snapshot remains current.
 */
export class RustSyntaxFactsService extends DisposableOwner {
  private cached: CachedSyntaxFacts | undefined;
  private disposed = false;

  constructor(private readonly syntax: ISyntaxApi) {
    super();
    this.defer(() => {
      this.disposed = true;
      this.cached = undefined;
    });
  }

  async analyze(languageId: string, snapshot: TextSnapshot, signal: AbortSignal): Promise<SyntaxAnalyzeResult | undefined> {
    this.ensureAlive();
    const language = syntaxLanguageForAsterLanguage(languageId);
    if (!language) return undefined;
    const text = snapshot.getText();
    if (new TextEncoder().encode(text).byteLength > MAX_SYNTAX_INPUT_BYTES) return undefined;
    const key = `${language}\u0000${snapshot.version}\u0000${text}`;
    let cached = this.cached;
    if (!cached || cached.key !== key) {
      const promise = this.syntax.analyze(Object.freeze({ language, revision: snapshot.version, text }));
      cached = Object.freeze({ key, promise });
      this.cached = cached;
      void promise.catch(() => {
        if (this.cached === cached) this.cached = undefined;
      });
    }
    const result = await raceCancellation(cached.promise, signal, "Rust syntax request was cancelled");
    if (result.revision !== snapshot.version) {
      throw new Error("Rust syntax result does not match the requested Aster model revision");
    }
    return result;
  }

  private ensureAlive(): void {
    if (this.disposed) throw new ReferenceError("Rust syntax facts service is already disposed");
  }
}

/** Runs parser facts through Aster's existing token and diagnostic result gates. */
export class RustSyntaxWorker implements SyntaxWorker, LanguageWorkerModelSynchronizer, LanguageWorkerResultSettler {
  constructor(private readonly facts: RustSyntaxFactsService, private readonly fallback: SyntaxWorker) {}

  async run(request: LanguageWorkerRequest<typeof SYNTAX_TOKEN_LANE | typeof SYNTAX_DIAGNOSTIC_LANE, SyntaxRequest>, signal: AbortSignal): Promise<SyntaxResult> {
    const result = await this.facts.analyze(request.payload.languageId, request.snapshot, signal);
    if (!result) return this.fallback.run(request, signal);
    signal.throwIfAborted();
    switch (request.lane) {
      case SYNTAX_TOKEN_LANE:
        return Object.freeze({ lane: SYNTAX_TOKEN_LANE, value: projectRustSyntaxTokens(result, request.snapshot) });
      case SYNTAX_DIAGNOSTIC_LANE:
        return Object.freeze({ lane: SYNTAX_DIAGNOSTIC_LANE, value: projectRustSyntaxDiagnostics(result, request.snapshot) });
    }
  }

  synchronizeModel(change: Parameters<LanguageWorkerModelSynchronizer["synchronizeModel"]>[0]): void {
    const synchronizer = this.fallback as Partial<LanguageWorkerModelSynchronizer>;
    synchronizer.synchronizeModel?.(change);
  }

  settleResult(requestId: number, disposition: LanguageWorkerResultDisposition): void {
    const settler = this.fallback as Partial<LanguageWorkerResultSettler>;
    settler.settleResult?.(requestId, disposition);
  }

  dispose(): void {
    this.fallback.dispose();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }
}

/** Supplies parser-derived declarations only after registered richer symbol providers decline. */
export class RustSyntaxDocumentSymbolProvider implements LanguageDocumentSymbolProvider {
  readonly providerId = "zeta.rustSyntax";
  readonly languageIds = Object.freeze(["javascript", "javascriptreact", "json", "jsonc", "rust", "shell", "typescript", "typescriptreact"]);

  constructor(private readonly facts: RustSyntaxFactsService) {}

  async provideDocumentSymbols(request: { readonly snapshot: TextSnapshot; readonly languageId: string; readonly signal: AbortSignal }): Promise<readonly LanguageDocumentSymbol[]> {
    const result = await this.facts.analyze(request.languageId, request.snapshot, request.signal);
    if (!result || request.signal.aborted) return Object.freeze([]);
    return projectRustSyntaxSymbols(result, request.snapshot);
  }
}

export function syntaxLanguageForAsterLanguage(languageId: string): "javascript" | "javascriptreact" | "json" | "jsonc" | "rust" | "shell" | "typescript" | "typescriptreact" | undefined {
  switch (languageId) {
    case "javascript": return "javascript";
    case "javascriptreact": return "javascriptreact";
    case "json": return "json";
    case "jsonc": return "jsonc";
    case "rust": return "rust";
    case "shell": return "shell";
    case "typescript": return "typescript";
    case "typescriptreact": return "typescriptreact";
    default: return undefined;
  }
}

export function projectRustSyntaxTokens(result: SyntaxAnalyzeResult, snapshot: TextSnapshot): LanguageTokenResult {
  assertMatchingRevision(result, snapshot);
  const lines = snapshotLines(snapshot);
  const tokens: LanguageToken[] = [];
  for (const token of result.tokens) {
    for (const range of projectSingleLineRanges(token.range, lines)) {
      overlayToken(tokens, Object.freeze({
        range,
        tokenType: syntaxTokenType(token),
        modifiers: Object.freeze([]),
      }));
    }
  }
  return Object.freeze({ tokens: Object.freeze(tokens) });
}

function overlayToken(tokens: LanguageToken[], incoming: LanguageToken): void {
  const retained: LanguageToken[] = [];
  for (const token of tokens) {
    if (token.range.end.compareTo(incoming.range.start) <= 0 || incoming.range.end.compareTo(token.range.start) <= 0) {
      retained.push(token);
      continue;
    }
    if (token.range.start.compareTo(incoming.range.start) < 0) retained.push(tokenWithRange(token, TextRange.from(token.range.start, incoming.range.start)));
    if (incoming.range.end.compareTo(token.range.end) < 0) retained.push(tokenWithRange(token, TextRange.from(incoming.range.end, token.range.end)));
  }
  retained.push(incoming);
  retained.sort((left, right) => left.range.start.compareTo(right.range.start) || left.range.end.compareTo(right.range.end));
  tokens.splice(0, tokens.length, ...retained);
}

function tokenWithRange(token: LanguageToken, range: TextRange): LanguageToken {
  return Object.freeze({
    range,
    tokenType: token.tokenType,
    modifiers: token.modifiers,
    ...(token.languageId === undefined ? {} : { languageId: token.languageId }),
    ...(token.balancedBrackets === false ? { balancedBrackets: false as const } : {}),
    ...(token.presentation === undefined ? {} : { presentation: token.presentation }),
  });
}

export function projectRustSyntaxDiagnostics(result: SyntaxAnalyzeResult, snapshot: TextSnapshot): LanguageDiagnosticResult {
  assertMatchingRevision(result, snapshot);
  const lines = snapshotLines(snapshot);
  return Object.freeze({
    diagnostics: Object.freeze(result.diagnostics.flatMap(diagnostic => projectRustSyntaxDiagnostic(diagnostic, lines))),
  });
}

export function projectRustSyntaxSymbols(result: SyntaxAnalyzeResult, snapshot: TextSnapshot): readonly LanguageDocumentSymbol[] {
  assertMatchingRevision(result, snapshot);
  const lines = snapshotLines(snapshot);
  return Object.freeze(result.symbols.map(symbol => projectRustSyntaxSymbol(symbol, lines)));
}

function projectRustSyntaxDiagnostic(diagnostic: SyntaxDiagnostic, lines: readonly string[]) {
  const range = projectRange(diagnostic.range, lines);
  return Object.freeze({
    range,
    severity: LanguageDiagnosticSeverity.Error,
    message: diagnostic.kind === "missing" ? "Missing required syntax" : "Syntax error",
    code: diagnostic.kind === "missing" ? "syntax-missing" : "syntax-error",
    source: "zeta-syntax",
  });
}

function projectRustSyntaxSymbol(symbol: SyntaxSymbol, lines: readonly string[]): LanguageDocumentSymbol {
  if (typeof symbol.name !== "string" || symbol.name.trim().length === 0) {
    throw new TypeError("Rust syntax symbol must have a non-empty name");
  }
  return Object.freeze({
    name: symbol.name,
    kind: symbol.kind,
    range: projectRange(symbol.range, lines),
    selectionRange: projectRange(symbol.selectionRange, lines),
  });
}

function syntaxTokenType(token: SyntaxToken): string {
  switch (token.kind) {
    case "attribute": return "modifier";
    case "comment": return "comment";
    case "constant": return "variable";
    case "constructor": return "class";
    case "embedded": return "string";
    case "function": return "function";
    case "keyword": return "keyword";
    case "label": return "variable";
    case "module": return "namespace";
    case "number": return "number";
    case "operator": return "operator";
    case "property": return "property";
    case "punctuation": return "punctuation";
    case "string": return "string";
    case "type": return "type";
    case "variable": return "variable";
  }
}

function projectSingleLineRanges(range: SyntaxToken["range"], lines: readonly string[]): readonly TextRange[] {
  const projected = projectRange(range, lines);
  if (projected.empty) return Object.freeze([]);
  const ranges: TextRange[] = [];
  for (let lineIndex = projected.start.lineIndex; lineIndex <= projected.end.lineIndex; lineIndex += 1) {
    const startColumn = lineIndex === projected.start.lineIndex ? projected.start.columnIndex : 0;
    const endColumn = lineIndex === projected.end.lineIndex ? projected.end.columnIndex : lines[lineIndex]!.length;
    if (endColumn > startColumn) ranges.push(TextRange.from(TextPosition.at(lineIndex, startColumn), TextPosition.at(lineIndex, endColumn)));
  }
  return Object.freeze(ranges);
}

function projectRange(range: SyntaxToken["range"], lines: readonly string[]): TextRange {
  return TextRange.from(projectPosition(range.start, lines), projectPosition(range.end, lines));
}

function projectPosition(position: { readonly lineIndex: number; readonly columnIndex: number }, lines: readonly string[]): TextPosition {
  if (!Number.isSafeInteger(position.lineIndex) || !Number.isSafeInteger(position.columnIndex) || position.lineIndex < 0 || position.columnIndex < 0 || position.lineIndex >= lines.length || position.columnIndex > lines[position.lineIndex]!.length) {
    throw new RangeError("Rust syntax range is outside its Aster snapshot");
  }
  return TextPosition.at(position.lineIndex, position.columnIndex);
}

function snapshotLines(snapshot: TextSnapshot): readonly string[] {
  const text = snapshot.getText();
  const lines = text.split("\n");
  if (text.length !== snapshot.length || lines.length !== snapshot.lineCount) {
    throw new Error("Aster syntax snapshot metadata is inconsistent");
  }
  return Object.freeze(lines);
}

function assertMatchingRevision(result: SyntaxAnalyzeResult, snapshot: TextSnapshot): void {
  if (result.revision !== snapshot.version) {
    throw new Error("Rust syntax result does not match the requested Aster snapshot");
  }
}
