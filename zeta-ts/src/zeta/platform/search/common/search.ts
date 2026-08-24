import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type WorkspaceSearchPatternKind = "literal" | "regex";
export type WorkspaceSearchCaseSensitivity = "smart" | "sensitive" | "insensitive";

export interface WorkspaceSearchMatchRange {
	start: number;
	end: number;
}

export interface WorkspaceSearchMatch {
	readonly workspaceFolderId?: string;
	readonly workspaceFolderName?: string;
	readonly path: string;
	readonly lineNumber: number;
	readonly preview: string;
	readonly ranges: readonly WorkspaceSearchMatchRange[];
}

/** A content query applied to the current workspace. */
export interface IWorkspaceSearchQuery {
	readonly text: string;
	readonly patternKind: WorkspaceSearchPatternKind;
	readonly caseSensitivity: WorkspaceSearchCaseSensitivity;
	readonly includePatterns: readonly string[];
	readonly excludePatterns: readonly string[];
	readonly maxResults?: number;
}

/** Terminal metadata returned after all available result batches are consumed. */
export interface IWorkspaceSearchComplete {
	readonly resultCount: number;
	readonly limitHit: boolean;
	readonly error: string | undefined;
}

/** Runtime controls for one cancellable workspace search. */
export interface IWorkspaceSearchOptions {
	readonly signal?: AbortSignal;
	readonly onProgress?: (
		matches: readonly WorkspaceSearchMatch[],
	) => void;
}

/** Renderer-facing workspace search lifecycle independent of Electron IPC. */
export interface IWorkspaceSearchService {
	search(query: IWorkspaceSearchQuery, options?: IWorkspaceSearchOptions): Promise<IWorkspaceSearchComplete>;
}

export const IWorkspaceSearchService = createServiceIdentifier<IWorkspaceSearchService>("workspaceSearchService");
