import type { CodebaseSymbolsSearchResult, CodebaseSymbolsStatusResult, DocumentOverlayStatusResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ICodebaseSymbolsApi } from "../common/codebaseSymbolsApi.js";

export function createCodebaseSymbolsApi(): ICodebaseSymbolsApi {
	return {
		status: () => invoke<CodebaseSymbolsStatusResult>("zeta:codebase-symbols:status"),
		search: params => invoke<CodebaseSymbolsSearchResult>("zeta:codebase-symbols:search", params),
		synchronize: params => invoke<DocumentOverlayStatusResult>("zeta:codebase-symbols:document-synchronize", params),
		close: params => invoke<DocumentOverlayStatusResult>("zeta:codebase-symbols:document-close", params),
	};
}
