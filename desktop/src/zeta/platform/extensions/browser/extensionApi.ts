import type { ExtensionCatalogReload, IExtensionApi } from "../common/extensionApi.js";
import { normalizeExtensionCatalog } from "../common/extensionApi.js";
import type { ExtensionListResult, ExtensionResourceOpenResult, ResourceReadResult } from "../../../../../generated/app-server/types.js";
import type { IResourceApi } from "../../app-server/common/appServerApi.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";

export function createDisconnectedExtensionApi(unavailable: UnavailableOperation): IExtensionApi {
  return {
    list: () => unavailable("extensions.list"),
    readResource: () => unavailable("extensions.readResource"),
  };
}

export function createViteDevExtensionApi(connection: ViteDevAppServerConnection, resourceApi: IResourceApi): IExtensionApi {
  return {
    list: async (reload: ExtensionCatalogReload) => normalizeExtensionCatalog(await viteDevRequest(connection, "extensions/list", { reload })),
    readResource: (extensionId, path) => readExtensionResource(connection, resourceApi, extensionId, path),
  };
}

async function readExtensionResource(connection: ViteDevAppServerConnection, resourceApi: IResourceApi, extensionId: string, path: string): Promise<Uint8Array> {
  const opened = await viteDevRequest(connection, "extensions/resource/open", { extensionId, path }) as ExtensionResourceOpenResult;
  const resource = opened.resource;
  const chunks: Uint8Array[] = [];
  let offset = 0;
  try {
    while (offset < resource.size) {
      const chunk = await resourceApi.read({ resourceId: resource.resourceId, offset, maxBytes: Math.min(262_144, resource.size - offset) });
      const bytes = decodeBase64(chunk);
      if (chunk.offset !== offset || chunk.decodedLength !== bytes.length || bytes.length === 0) throw new Error("Extension resource response is inconsistent");
      chunks.push(bytes);
      offset += bytes.length;
      if (chunk.eof !== (offset === resource.size)) throw new Error("Extension resource EOF marker is inconsistent");
    }
    return joinBytes(chunks, resource.size);
  } finally {
    await resourceApi.release({ resourceId: resource.resourceId });
  }
}

function decodeBase64(result: ResourceReadResult): Uint8Array {
  const binary = atob(result.dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

function joinBytes(chunks: readonly Uint8Array[], size: number): Uint8Array {
  const result = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  if (offset !== size) throw new Error("Extension resource byte count is inconsistent");
  return result;
}
