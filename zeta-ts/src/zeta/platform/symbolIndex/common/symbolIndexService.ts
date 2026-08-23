import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type SymbolIndexState = "empty" | "indexing" | "ready" | "stale" | "failed";
export type SymbolIndexSymbolKind = "constant" | "enum" | "field" | "function" | "macro" | "method" | "module" | "static" | "struct" | "trait" | "type" | "variable";

export interface SymbolIndexPosition {
	readonly lineIndex: number;
	readonly columnIndex: number;
}

export interface SymbolIndexRange {
	readonly start: SymbolIndexPosition;
	readonly end: SymbolIndexPosition;
}

export interface SymbolIndexStatus {
	readonly state: SymbolIndexState;
	readonly rootId: string;
	readonly generation: number;
	readonly sourceGeneration: number;
	readonly indexedSourceCount: number;
	readonly indexedSymbolCount: number;
	readonly symbolLimitHit: boolean;
}

export interface SymbolIndexMatch {
	readonly name: string;
	readonly kind: SymbolIndexSymbolKind;
	readonly containerName?: string;
	readonly path: string;
	readonly language: string;
	readonly sourceRevision: string;
	readonly declarationRange: SymbolIndexRange;
	readonly selectionRange: SymbolIndexRange;
	readonly score: number;
	readonly matchedIndices: readonly number[];
}

export interface SymbolIndexSearchResult {
	readonly status: SymbolIndexStatus;
	readonly matches: readonly SymbolIndexMatch[];
	readonly discardedStaleMatchCount: number;
}

/** Frontend-owned local declaration search contract for the active Workspace. */
export interface ISymbolIndexService {
	status(signal?: AbortSignal): Promise<SymbolIndexStatus>;
	search(query: string, maxResults: number, signal?: AbortSignal): Promise<SymbolIndexSearchResult>;
}

export const ISymbolIndexService = createServiceIdentifier<ISymbolIndexService>("symbolIndexService");
