import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IConnectorApi } from "../common/connectorApi.js";

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

export function createViteDevConnectorApi(connection: ViteDevAppServerConnection): IConnectorApi {
  return {
    list: () => viteDevRequest(connection, "connector/list", {}),
    connectApiToken: params => viteDevRequest(connection, "connector/connect/apiToken", params),
    connectOAuth: () => Promise.reject(new Error("Connector OAuth callback hosting is unavailable in this browser host")),
    disconnect: params => viteDevRequest(connection, "connector/disconnect", params),
    refreshOAuth: async connectorId => { await viteDevRequest(connection, "connector/oauth/refresh", { connectorId }); },
    revokeOAuth: params => viteDevRequest(connection, "connector/oauth/revoke", params),
  };
}
