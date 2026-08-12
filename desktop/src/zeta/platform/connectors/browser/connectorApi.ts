import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IConnectorApi } from "../common/connectorApi.js";

export function createDisconnectedConnectorApi(unavailable: UnavailableOperation): IConnectorApi {
  return {
    list: () => unavailable("connectors.list"),
    connectApiToken: () => unavailable("connectors.connectApiToken"),
    disconnect: () => unavailable("connectors.disconnect"),
  };
}

export function createViteDevConnectorApi(connection: ViteDevAppServerConnection): IConnectorApi {
  return {
    list: () => viteDevRequest(connection, "connector/list", {}),
    connectApiToken: params => viteDevRequest(connection, "connector/connect/apiToken", params),
    disconnect: params => viteDevRequest(connection, "connector/disconnect", params),
  };
}
