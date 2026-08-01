import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IFileApi } from "../common/fileApi.js";

export function createDisconnectedFileApi(unavailable: UnavailableOperation): IFileApi {
  return {
    getMetadata: () => unavailable("fs.getMetadata"),
    readDirectory: () => unavailable("fs.readDirectory"),
    readFile: () => unavailable("fs.readFile"),
  };
}

export function createViteDevFileApi(connection: ViteDevAppServerConnection): IFileApi {
  return {
    getMetadata: (params) => viteDevRequest(connection, "fs/getMetadata", params),
    readDirectory: (params) => viteDevRequest(connection, "fs/readDirectory", params),
    readFile: (params) => viteDevRequest(connection, "fs/readFile", params),
  };
}
