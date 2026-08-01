import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IWorkspaceSearchApi } from "../common/searchApi.js";

export function createDisconnectedWorkspaceSearchApi(unavailable: UnavailableOperation): IWorkspaceSearchApi {
  return {
    start: () => unavailable("workspaceSearch.start"),
    read: () => unavailable("workspaceSearch.read"),
    cancel: () => unavailable("workspaceSearch.cancel"),
  };
}

export function createViteDevWorkspaceSearchApi(connection: ViteDevAppServerConnection): IWorkspaceSearchApi {
  return {
    start: (params) => viteDevRequest(connection, "workspace/search/start", params),
    read: (params) => viteDevRequest(connection, "workspace/search/read", params),
    cancel: (params) => voidResult(viteDevRequest(connection, "workspace/search/cancel", params)),
  };
}
