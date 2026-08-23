import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IConnectorApi } from "../common/connectorApi.js";
import type { IClipboardService } from "../../clipboard/common/clipboardService.js";
import type { IOpenerService } from "../../opener/common/openerService.js";

export interface BrowserConnectorHostServices {
	readonly openerService: IOpenerService;
	readonly clipboardService: IClipboardService;
}

export function createDisconnectedConnectorApi(unavailable: UnavailableOperation): IConnectorApi {
	return {
		list: () => unavailable("connectors.list"),
		connectApiToken: () => unavailable("connectors.connectApiToken"),
		connectOAuth: () => unavailable("connectors.connectOAuth"),
		disconnect: () => unavailable("connectors.disconnect"),
		refreshOAuth: () => unavailable("connectors.refreshOAuth"),
		revokeOAuth: () => unavailable("connectors.revokeOAuth"),
	};
}

export function createViteDevConnectorApi(connection: ViteDevAppServerConnection, hostServices: BrowserConnectorHostServices): IConnectorApi {
	return {
		list: () => viteDevRequest(connection, "connector/list", {}),
		connectApiToken: params => viteDevRequest(connection, "connector/connect/apiToken", params),
		connectOAuth: params => connectDeviceOAuth(connection, params, hostServices),
		disconnect: params => viteDevRequest(connection, "connector/disconnect", params),
		refreshOAuth: async connectorId => { await viteDevRequest(connection, "connector/oauth/refresh", { connectorId }); },
		revokeOAuth: params => viteDevRequest(connection, "connector/oauth/revoke", params),
	};
}

async function connectDeviceOAuth(connection: ViteDevAppServerConnection, params: Parameters<IConnectorApi["connectOAuth"]>[0], hostServices: BrowserConnectorHostServices) {
	const catalog = await viteDevRequest(connection, "connector/list", {});
	const connector = catalog.connectors.find(candidate => candidate.id === params.connectorId);
	if (!connector?.oauthMethods.includes("device")) throw new Error("Connector device OAuth is unavailable in this browser host");
	const started = await viteDevRequest(connection, "connector/connect/oauth/device/start", params);
	let completed = false;
	try {
		await hostServices.openerService.openExternal(started.verificationUri);
		await hostServices.clipboardService.writeText(started.userCode);
		let waitSeconds = started.pollIntervalSeconds;
		for (;;) {
			await new Promise(resolve => setTimeout(resolve, Math.min(waitSeconds, 30) * 1_000));
			const result = await viteDevRequest(connection, "connector/connect/oauth/device/poll", { flowId: started.flowId });
			if (result.status === "connected") {
				completed = true;
				return result.command;
			}
			waitSeconds = result.retryAfterSeconds;
		}
	} finally {
		if (!completed) await viteDevRequest(connection, "connector/connect/oauth/device/cancel", { flowId: started.flowId }).catch(() => undefined);
	}
}
