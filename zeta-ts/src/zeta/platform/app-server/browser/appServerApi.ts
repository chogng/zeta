import type { IAppServerApi, IResourceApi, IServerEventApi } from "../common/appServerApi.js";
import { inertSubscription, type UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ViteDevAppServerConnection } from "./viteDevConnection.js";
import { viteDevRequest, voidResult } from "./viteDevRequest.js";

export function createDisconnectedAppServerApi(unavailable: UnavailableOperation): IAppServerApi {
	return {
		getConnectionState: () => Promise.resolve("stopped"),
		getSlashCommands: () => unavailable("appServer.getSlashCommands"),
		onConnectionState: inertSubscription,
	};
}

export function createDisconnectedResourceApi(unavailable: UnavailableOperation): IResourceApi {
	return {
		metadata: () => unavailable("resource.metadata"),
		read: () => unavailable("resource.read"),
		release: () => unavailable("resource.release"),
	};
}

export function createDisconnectedServerEventApi(): IServerEventApi {
	return { subscribe: inertSubscription };
}

export function createViteDevAppServerApi(connection: ViteDevAppServerConnection): IAppServerApi {
	return {
		getConnectionState: () => Promise.resolve(connection.state),
		getSlashCommands: () => Promise.resolve(connection.slashCommands),
		onConnectionState: (listener) => connection.onStateChange(listener),
	};
}

export function createViteDevResourceApi(connection: ViteDevAppServerConnection): IResourceApi {
	return {
		metadata: (params) => viteDevRequest(connection, "resource/metadata", params),
		read: (params) => viteDevRequest(connection, "resource/read", params),
		release: (params) => voidResult(viteDevRequest(connection, "resource/release", params)),
	};
}

export function createViteDevServerEventApi(connection: ViteDevAppServerConnection): IServerEventApi {
	return { subscribe: (listener) => connection.onNotification(listener) };
}
