import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ITypstApi } from "../common/typstApi.js";

export function createDisconnectedTypstApi(unavailable: UnavailableOperation): ITypstApi {
  return { compile: () => unavailable("typst.compile") };
}

export function createViteDevTypstApi(connection: ViteDevAppServerConnection): ITypstApi {
  return { compile: (params) => viteDevRequest(connection, "document/typst/compile", params) };
}
