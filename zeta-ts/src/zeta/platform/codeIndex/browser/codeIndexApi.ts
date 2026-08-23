import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ICodeIndexApi } from "../common/codeIndexApi.js";

export function createDisconnectedCodeIndexApi(unavailable: UnavailableOperation): ICodeIndexApi {
	return {
		readConfig: () => unavailable("codeIndex.readConfig"),
		configureProvider: () => unavailable("codeIndex.configureProvider"),
		configure: () => unavailable("codeIndex.configure"),
		authorize: () => unavailable("codeIndex.authorize"),
		revoke: () => unavailable("codeIndex.revoke"),
		status: () => unavailable("codeIndex.status"),
		cancel: () => unavailable("codeIndex.cancel"),
		retry: () => unavailable("codeIndex.retry"),
	};
}

export function createViteDevCodeIndexApi(connection: ViteDevAppServerConnection): ICodeIndexApi {
	return {
		readConfig: () => viteDevRequest(connection, "config/read", {}),
		configureProvider: params => viteDevRequest(connection, "provider/configure", params),
		configure: params => viteDevRequest(connection, "workspace/codeIndex/semantic/configure", params),
		authorize: params => viteDevRequest(connection, "workspace/codeIndex/semantic/authorize", params),
		revoke: params => viteDevRequest(connection, "workspace/codeIndex/semantic/revoke", params),
		status: () => viteDevRequest(connection, "workspace/codeIndex/status", {}),
		cancel: () => viteDevRequest(connection, "workspace/codeIndex/semantic/cancel", {}),
		retry: () => viteDevRequest(connection, "workspace/codeIndex/semantic/retry", {}),
	};
}
