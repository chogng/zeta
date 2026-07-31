import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type SyntaxLanguage = "json" | "jsonc" | "rust";
export type SyntaxTokenType = "attribute" | "comment" | "constant" | "constructor" | "embedded" | "function" | "keyword" | "label" | "module" | "number" | "operator" | "property" | "string" | "type" | "variable";
export type SyntaxDocumentSymbolKind = "constant" | "enum" | "field" | "function" | "macro" | "method" | "module" | "static" | "struct" | "trait" | "type" | "variable";
export type SyntaxDiagnosticSeverity = "error";

export interface SyntaxPosition {
  readonly line: number;
  readonly character: number;
}

export interface SyntaxRange {
  readonly start: SyntaxPosition;
  readonly end: SyntaxPosition;
}

export interface SyntaxTokenData {
  readonly legend: readonly SyntaxTokenType[];
  readonly data: readonly number[];
}

export interface SyntaxDocumentSymbol {
  readonly name: string;
  readonly kind: SyntaxDocumentSymbolKind;
  readonly range: SyntaxRange;
  readonly selectionRange: SyntaxRange;
}

export interface SyntaxDiagnostic {
  readonly range: SyntaxRange;
  readonly severity: SyntaxDiagnosticSeverity;
  readonly message: string;
  readonly source: string;
}

export interface SyntaxDocumentOpenRequest {
  readonly documentId: string;
  readonly documentUri: string;
  readonly language: SyntaxLanguage;
  readonly revision: number;
  readonly text: string;
}

export interface SyntaxTextEdit {
  readonly startUtf16: number;
  readonly endUtf16: number;
  readonly text: string;
}

export interface SyntaxDocumentChangeRequest {
  readonly documentId: string;
  readonly previousRevision: number;
  readonly revision: number;
  readonly edits: readonly SyntaxTextEdit[];
}

export interface SyntaxDocumentCloseRequest {
  readonly documentId: string;
}

export interface SyntaxAnalysisSnapshot {
  readonly revision: number;
  readonly resultId: string;
  readonly hasErrors: boolean;
  readonly tokens: SyntaxTokenData;
  readonly foldingRanges: readonly SyntaxRange[];
  readonly symbols: readonly SyntaxDocumentSymbol[];
  readonly diagnostics: readonly SyntaxDiagnostic[];
}

/**
 * Provides revisioned syntax snapshots independently of the transport that
 * executes the analysis.
 *
 * Implementations own the open/change/close document lifecycle and must keep
 * every returned snapshot tied to the requested document revision.
 */
export interface ISyntaxAnalysisService {
  open(request: SyntaxDocumentOpenRequest): Promise<SyntaxAnalysisSnapshot>;
  change(request: SyntaxDocumentChangeRequest): Promise<SyntaxAnalysisSnapshot>;
  close(request: SyntaxDocumentCloseRequest): Promise<void>;
}

export const ISyntaxAnalysisService = createServiceIdentifier<ISyntaxAnalysisService>("syntaxAnalysisService");
