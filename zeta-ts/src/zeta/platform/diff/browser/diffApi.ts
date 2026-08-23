import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IDiffApi } from "../common/diffApi.js";

export function createDisconnectedDiffApi(unavailable: UnavailableOperation): IDiffApi {
  return {
    compute: () => unavailable("diff.compute"),
  };
}

export function createViteDevDiffApi(connection: ViteDevAppServerConnection): IDiffApi {
  return {
    compute: request => viteDevRequest(connection, "diff/compute", request),
  };
}
