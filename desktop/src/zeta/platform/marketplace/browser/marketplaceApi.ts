import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IMarketplaceApi } from "../common/marketplaceApi.js";

export function createDisconnectedMarketplaceApi(unavailable: UnavailableOperation): IMarketplaceApi {
  return { search: () => unavailable("marketplace.search"), get: () => unavailable("marketplace.get"), download: () => unavailable("marketplace.download"), install: () => unavailable("marketplace.install"), update: () => unavailable("marketplace.update"), uninstall: () => unavailable("marketplace.uninstall"), listInstalled: () => unavailable("marketplace.listInstalled"), acquireCapability: () => unavailable("marketplace.acquireCapability"), releaseCapability: () => unavailable("marketplace.releaseCapability"), openResource: () => unavailable("marketplace.openResource") };
}

export function createViteDevMarketplaceApi(connection: ViteDevAppServerConnection): IMarketplaceApi {
  return {
    search: params => viteDevRequest(connection, "marketplace/search", params),
    get: params => viteDevRequest(connection, "marketplace/get", params),
    download: params => viteDevRequest(connection, "marketplace/download", params),
    install: params => viteDevRequest(connection, "marketplace/install", params),
    update: params => viteDevRequest(connection, "marketplace/update", params),
    uninstall: params => voidResult(viteDevRequest(connection, "marketplace/uninstall", params)),
    listInstalled: () => viteDevRequest(connection, "marketplace/listInstalled", {}),
    acquireCapability: params => viteDevRequest(connection, "marketplace/acquireCapability", params),
    releaseCapability: params => voidResult(viteDevRequest(connection, "marketplace/releaseCapability", params)),
    openResource: params => viteDevRequest(connection, "marketplace/openResource", params),
  };
}
