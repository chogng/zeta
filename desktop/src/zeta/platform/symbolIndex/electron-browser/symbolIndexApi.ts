import type { SymbolIndexSearchResult, SymbolIndexStatusResult, WorkspaceDocumentOverlayStatusResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ISymbolIndexApi } from "../common/symbolIndexApi.js";

export function createSymbolIndexApi(): ISymbolIndexApi {
  return {
    status: () => invoke<SymbolIndexStatusResult>("zeta:symbol-index:status"),
    search: params => invoke<SymbolIndexSearchResult>("zeta:symbol-index:search", params),
    synchronize: params => invoke<WorkspaceDocumentOverlayStatusResult>("zeta:symbol-index:document-synchronize", params),
    close: params => invoke<WorkspaceDocumentOverlayStatusResult>("zeta:symbol-index:document-close", params),
  };
}
