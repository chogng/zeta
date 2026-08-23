import type { SymbolIndexSearchParams, SymbolIndexSearchResult, SymbolIndexStatusResult, WorkspaceDocumentOverlayCloseParams, WorkspaceDocumentOverlayStatusResult, WorkspaceDocumentOverlaySynchronizeParams } from "../../../../../generated/app-server/types.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

/** Transport-only symbol-index operations. Product consumers use ISymbolIndexService. */
export interface ISymbolIndexApi {
	status(): Promise<SymbolIndexStatusResult>;
	search(params: SymbolIndexSearchParams): Promise<SymbolIndexSearchResult>;
	synchronize(params: WorkspaceDocumentOverlaySynchronizeParams): Promise<WorkspaceDocumentOverlayStatusResult>;
	close(params: WorkspaceDocumentOverlayCloseParams): Promise<WorkspaceDocumentOverlayStatusResult>;
}

export const ISymbolIndexApi = createServiceIdentifier<ISymbolIndexApi>("symbolIndexApi");
