import type { IExtensionHostApi } from "../common/extensionHostApi.js";
import { invokeExtensionHost, normalizeExtensionHostChanged, normalizeExtensionHostSnapshot } from "../common/extensionHostApi.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import { inertSubscription } from "../../renderer/browser/disconnectedHost.js";

export function createDisconnectedExtensionHostApi(unavailable: UnavailableOperation): IExtensionHostApi {
	return {
		isAvailable: () => Promise.resolve(false),
		list: () => unavailable("extensionHost.list"),
		reconcile: () => unavailable("extensionHost.reconcile"),
		invoke: () => unavailable("extensionHost.invoke"),
		getConnectionState: () => Promise.resolve("stopped"),
		onDidChange: inertSubscription,
		onConnectionState: inertSubscription,
	};
}

export function createViteDevExtensionHostApi(connection: ViteDevAppServerConnection): IExtensionHostApi {
	const transport = {
		start: (request: Parameters<IExtensionHostApi["invoke"]>[0]) => viteDevRequest(connection, "extensionHost/invoke/start", request),
		read: (invocationId: string) => viteDevRequest(connection, "extensionHost/invoke/read", { invocationId }),
		cancel: (invocationId: string) => viteDevRequest(connection, "extensionHost/invoke/cancel", { invocationId }),
	};
	return {
		isAvailable: () => Promise.resolve(connection.capabilities?.extensionHost === true),
		list: async () => normalizeExtensionHostSnapshot(await viteDevRequest(connection, "extensionHost/list", {})),
		reconcile: async mode => normalizeExtensionHostSnapshot(await viteDevRequest(connection, "extensionHost/reconcile", { mode })),
		invoke: (request, signal) => invokeExtensionHost(transport, request, signal),
		getConnectionState: () => Promise.resolve(connection.state),
		onDidChange: listener => connection.onNotification(event => {
			if (event.method === "extensionHost/changed") listener(normalizeExtensionHostChanged(event.params));
		}),
		onConnectionState: listener => connection.onStateChange(listener),
	};
}
