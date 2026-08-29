import type { CodebaseSymbolsSearchParams, CodebaseSymbolsSearchResult, CodebaseSymbolsStatusResult, WorkspaceDocumentOverlayCloseParams, WorkspaceDocumentOverlayStatusResult, WorkspaceDocumentOverlaySynchronizeParams } from "../../../../../generated/app-server/types.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** Transport-only codebase-symbols operations. Product consumers use ICodebaseSymbolsService. */
export interface ICodebaseSymbolsApi {
	status(): Promise<CodebaseSymbolsStatusResult>;
	search(params: CodebaseSymbolsSearchParams): Promise<CodebaseSymbolsSearchResult>;
	synchronize(params: WorkspaceDocumentOverlaySynchronizeParams): Promise<WorkspaceDocumentOverlayStatusResult>;
	close(params: WorkspaceDocumentOverlayCloseParams): Promise<WorkspaceDocumentOverlayStatusResult>;
}

export const ICodebaseSymbolsApi = createServiceIdentifier<ICodebaseSymbolsApi>("codebaseSymbolsApi");
