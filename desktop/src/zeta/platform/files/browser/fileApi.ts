import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IFileApi } from "../common/fileApi.js";

export function createDisconnectedFileApi(unavailable: UnavailableOperation): IFileApi {
  return {
    getMetadata: () => unavailable("fs.getMetadata"),
    readDirectory: () => unavailable("fs.readDirectory"),
    readFile: () => unavailable("fs.readFile"),
    readBinaryFile: () => unavailable("fs.readBinaryFile"),
    writeFile: () => unavailable("fs.writeFile"),
    createFile: () => unavailable("fs.createFile"),
    rename: () => unavailable("fs.rename"),
    delete: () => unavailable("fs.delete"),
  };
}

export function createViteDevFileApi(connection: ViteDevAppServerConnection): IFileApi {
  return {
    getMetadata: (params) => viteDevRequest(connection, "fs/getMetadata", params),
    readDirectory: (params) => viteDevRequest(connection, "fs/readDirectory", params),
    readFile: (params) => viteDevRequest(connection, "fs/readFile", params),
    readBinaryFile: (params) => viteDevRequest(connection, "fs/readBinaryFile", params),
    writeFile: (params) => viteDevRequest(connection, "fs/writeFile", params),
    createFile: (params) => viteDevRequest(connection, "fs/createFile", params),
    rename: (params) => voidResult(viteDevRequest(connection, "fs/rename", params)),
    delete: (params) => voidResult(viteDevRequest(connection, "fs/delete", params)),
  };
}
