import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IToolSearchApi } from "../common/toolSearchApi.js";

export function createDisconnectedToolSearchApi(unavailable: UnavailableOperation): IToolSearchApi {
  return {
    readConfig: () => unavailable("toolSearch.readConfig"),
    configure: () => unavailable("toolSearch.configure"),
  };
}

export function createViteDevToolSearchApi(connection: ViteDevAppServerConnection): IToolSearchApi {
  return {
    readConfig: () => viteDevRequest(connection, "config/read", {}),
    configure: params => viteDevRequest(connection, "toolSearch/configure", params),
  };
}
