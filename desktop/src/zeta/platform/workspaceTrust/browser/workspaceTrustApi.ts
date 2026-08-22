import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IWorkspaceTrustApi } from "../common/workspaceTrustApi.js";

export function createDisconnectedWorkspaceTrustApi(unavailable: UnavailableOperation): IWorkspaceTrustApi {
  return {
    list: () => unavailable("workspaceTrust.list"),
    read: () => unavailable("workspaceTrust.read"),
    set: () => unavailable("workspaceTrust.set"),
    forget: () => unavailable("workspaceTrust.forget"),
  };
}

export function createViteDevWorkspaceTrustApi(connection: ViteDevAppServerConnection): IWorkspaceTrustApi {
  return {
    list: () => viteDevRequest(connection, "workspace/trust/list", {}),
    read: params => viteDevRequest(connection, "workspace/trust/read", params),
    set: params => viteDevRequest(connection, "workspace/trust/set", params),
    forget: params => viteDevRequest(connection, "workspace/trust/forget", params),
  };
}
