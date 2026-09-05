import type { AppServerProtocolClient } from "../../app-server/browser/appServerProtocolClient.js";
import { appServerRequest, voidResult } from "../../app-server/browser/appServerRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IContentSearchApi } from "../common/searchApi.js";

export function createDisconnectedContentSearchApi(unavailable: UnavailableOperation): IContentSearchApi {
	return {
		start: () => unavailable("contentSearch.start"),
		read: () => unavailable("contentSearch.read"),
		cancel: () => unavailable("contentSearch.cancel"),
	};
}

export function createAppServerContentSearchApi(connection: AppServerProtocolClient): IContentSearchApi {
	return {
		start: (params) => appServerRequest(connection, "content/search/start", params),
		read: (params) => appServerRequest(connection, "content/search/read", params),
		cancel: (params) => voidResult(appServerRequest(connection, "content/search/cancel", params)),
	};
}
