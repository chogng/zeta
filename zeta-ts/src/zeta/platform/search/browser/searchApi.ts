import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IContentSearchApi } from "../common/searchApi.js";

export function createDisconnectedContentSearchApi(unavailable: UnavailableOperation): IContentSearchApi {
	return {
		start: () => unavailable("contentSearch.start"),
		read: () => unavailable("contentSearch.read"),
		cancel: () => unavailable("contentSearch.cancel"),
	};
}

export function createViteDevContentSearchApi(connection: ViteDevAppServerConnection): IContentSearchApi {
	return {
		start: (params) => viteDevRequest(connection, "content/search/start", params),
		read: (params) => viteDevRequest(connection, "content/search/read", params),
		cancel: (params) => voidResult(viteDevRequest(connection, "content/search/cancel", params)),
	};
}
