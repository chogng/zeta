import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IPluginApi } from "../common/pluginApi.js";

export function createDisconnectedPluginApi(unavailable: UnavailableOperation): IPluginApi {
  return {
    list: () => unavailable("plugins.list"),
    enable: () => unavailable("plugins.enable"),
    disable: () => unavailable("plugins.disable"),
    grant: () => unavailable("plugins.grant"),
    revokeGrant: () => unavailable("plugins.revokeGrant"),
    uninstall: () => unavailable("plugins.uninstall"),
  };
}

export function createViteDevPluginApi(connection: ViteDevAppServerConnection): IPluginApi {
  return {
    list: () => viteDevRequest(connection, "plugin/list", {}),
    enable: params => viteDevRequest(connection, "plugin/enable", params),
    disable: params => viteDevRequest(connection, "plugin/disable", params),
    grant: params => viteDevRequest(connection, "plugin/grant", params),
    revokeGrant: params => viteDevRequest(connection, "plugin/revokeGrant", params),
    uninstall: params => viteDevRequest(connection, "plugin/uninstall", params),
  };
}
