import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ISyntaxApi } from "../common/syntaxApi.js";

export function createDisconnectedSyntaxApi(unavailable: UnavailableOperation): ISyntaxApi {
	return {
		analyze: () => unavailable("syntax.analyze"),
		selectionRanges: () => unavailable("syntax.selectionRanges"),
	};
}

export function createViteDevSyntaxApi(connection: ViteDevAppServerConnection): ISyntaxApi {
	return {
		analyze: params => viteDevRequest(connection, "syntax/analyze", params),
		selectionRanges: params => viteDevRequest(connection, "syntax/selectionRanges", { ...params, ranges: [...params.ranges] }),
	};
}
