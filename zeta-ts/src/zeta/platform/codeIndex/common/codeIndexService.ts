import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export interface CodeIndexModelReference {
	readonly provider: string;
	readonly model: string;
}

export interface CodeIndexProviderConfiguration {
	readonly provider: string;
	readonly baseUrl?: string | null;
	readonly maxOutputTokens?: number | null;
	readonly modelContext?: Readonly<Record<string, { readonly contextWindow: number; readonly autoCompactTokenLimit?: number | null }>>;
}

export type SemanticCodeIndexSelection =
	| { readonly type: "disabled" }
	| { readonly type: "remote"; readonly models: { readonly embeddingModel: CodeIndexModelReference; readonly rerankModel?: CodeIndexModelReference | null } };

export type SemanticCodeIndexAutomaticContext = "off" | "firstInvocation";

/** Stable Settings identity for the aggregate semantic-code-search policy. */
export const SemanticCodeIndexSettingId = "codeIndex.semanticCodeIndex";

export interface CodeIndexConfigurationSnapshot {
	readonly revision: number;
	readonly generation: number;
	readonly providers: Readonly<Record<string, CodeIndexProviderConfiguration>>;
	readonly semanticCodeIndex: {
		readonly selection: SemanticCodeIndexSelection;
		readonly automaticContext: SemanticCodeIndexAutomaticContext;
		readonly activeWorkspaceAuthorized: boolean;
	};
}

export interface CodeIndexConfigurationCommandResult {
	readonly revision: number;
	readonly generation: number;
	readonly disposition: "updated" | "replayed";
}

export type SemanticCodeIndexState = "unavailable" | "idle" | "syncing" | "ready" | "stale" | "cancelled" | "failed";

export interface CodeIndexStatus {
	readonly state: "empty" | "indexing" | "ready" | "stale" | "failed";
	readonly rootId: string;
	readonly generation: number;
	readonly indexedFileCount: number;
	readonly indexedChunkCount: number;
	readonly indexedSourceBytes: number;
	readonly skippedFileCount: number;
	readonly truncatedFileCount: number;
	readonly fileLimitHit: boolean;
	readonly sourceBytesLimitHit: boolean;
	readonly semantic: {
		readonly state: SemanticCodeIndexState;
		readonly operationId: number | null;
		readonly targetGeneration: number;
		readonly publishedGeneration: number | null;
		readonly phase: string | null;
		readonly totalChunkCount: number;
		readonly processedChunkCount: number;
		readonly reusedEmbeddingCount: number;
		readonly embeddedChunkCount: number;
		readonly completedBatchCount: number;
		readonly totalBatchCount: number;
		readonly retryCount: number;
		readonly lastErrorCode: string | null;
	};
}

/** Frontend-owned semantic code-index configuration contract for the active Workspace. */
export interface ICodeIndexService {
	readConfig(): Promise<CodeIndexConfigurationSnapshot>;
	configureProvider(config: CodeIndexProviderConfiguration, expectedRevision: number): Promise<CodeIndexConfigurationCommandResult>;
	configure(selection: SemanticCodeIndexSelection, automaticContext: SemanticCodeIndexAutomaticContext, expectedRevision: number): Promise<CodeIndexConfigurationCommandResult>;
	authorize(expectedRevision: number): Promise<CodeIndexConfigurationCommandResult>;
	revoke(expectedRevision: number): Promise<CodeIndexConfigurationCommandResult>;
	status(): Promise<CodeIndexStatus>;
	cancel(): Promise<CodeIndexStatus>;
	retry(): Promise<CodeIndexStatus>;
}

export const ICodeIndexService = createServiceIdentifier<ICodeIndexService>("codeIndexService");
