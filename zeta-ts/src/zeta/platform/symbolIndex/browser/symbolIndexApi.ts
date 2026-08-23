import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ISymbolIndexApi } from "../common/symbolIndexApi.js";

export function createDisconnectedSymbolIndexApi(unavailable: UnavailableOperation): ISymbolIndexApi {
	return {
		status: () => unavailable("symbolIndex.status"),
		search: () => unavailable("symbolIndex.search"),
		synchronize: () => unavailable("symbolIndex.synchronize"),
		close: () => unavailable("symbolIndex.close"),
	};
}

export function createViteDevSymbolIndexApi(connection: ViteDevAppServerConnection): ISymbolIndexApi {
	return {
		status: () => viteDevRequest(connection, "workspace/symbolIndex/status", {}),
		search: params => viteDevRequest(connection, "workspace/symbolIndex/search", params),
		synchronize: params => viteDevRequest(connection, "workspace/codeIntelligence/document/synchronize", params),
		close: params => viteDevRequest(connection, "workspace/codeIntelligence/document/close", params),
	};
}
