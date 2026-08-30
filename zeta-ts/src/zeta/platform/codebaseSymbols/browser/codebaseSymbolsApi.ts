import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ICodebaseSymbolsApi } from "../common/codebaseSymbolsApi.js";

export function createDisconnectedCodebaseSymbolsApi(unavailable: UnavailableOperation): ICodebaseSymbolsApi {
	return {
		status: () => unavailable("codebaseSymbols.status"),
		search: () => unavailable("codebaseSymbols.search"),
		synchronize: () => unavailable("codebaseSymbols.synchronize"),
		close: () => unavailable("codebaseSymbols.close"),
	};
}

export function createViteDevCodebaseSymbolsApi(connection: ViteDevAppServerConnection): ICodebaseSymbolsApi {
	return {
		status: () => viteDevRequest(connection, "codebase/symbols/status", {}),
		search: params => viteDevRequest(connection, "codebase/symbols/search", params),
		synchronize: params => viteDevRequest(connection, "codeIntelligence/document/synchronize", params),
		close: params => viteDevRequest(connection, "codeIntelligence/document/close", params),
	};
}
