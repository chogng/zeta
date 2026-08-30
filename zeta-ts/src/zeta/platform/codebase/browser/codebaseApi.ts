import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ICodebaseApi } from "../common/codebaseApi.js";

export function createDisconnectedCodebaseApi(unavailable: UnavailableOperation): ICodebaseApi {
	return {
		readConfig: () => unavailable("codebase.readConfig"),
		configureProvider: () => unavailable("codebase.configureProvider"),
		configure: () => unavailable("codebase.configure"),
		status: () => unavailable("codebase.status"),
	};
}

export function createViteDevCodebaseApi(connection: ViteDevAppServerConnection): ICodebaseApi {
	return {
		readConfig: () => viteDevRequest(connection, "config/read", {}),
		configureProvider: params => viteDevRequest(connection, "provider/configure", params),
		configure: params => viteDevRequest(connection, "codebase/configure", params),
		status: () => viteDevRequest(connection, "codebase/status", {}),
	};
}
