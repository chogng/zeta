import type { ExtensionCatalogReload, ExtensionResourceRequest, IExtensionApi } from "../common/extensionApi.js";
import { normalizeExtensionCatalog, normalizeExtensionResourceChunk, normalizeExtensionResourceOpenResult, verifyExtensionResourceDigest } from "../common/extensionApi.js";
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
    readResource: request => readExtensionResource(connection, resourceApi, request),
  };
}

async function readExtensionResource(connection: ViteDevAppServerConnection, resourceApi: IResourceApi, request: ExtensionResourceRequest): Promise<Uint8Array> {
  const resource = normalizeExtensionResourceOpenResult(await viteDevRequest(connection, "extensions/resource/open", request));
  const chunks: Uint8Array[] = [];
  let offset = 0;
  try {
    while (offset < resource.size) {
      const chunk = normalizeExtensionResourceChunk(await resourceApi.read({ resourceId: resource.resourceId, offset, maxBytes: Math.min(262_144, resource.size - offset) }));
      const bytes = decodeBase64(chunk);
      if (chunk.resourceId !== resource.resourceId || chunk.offset !== offset || chunk.decodedLength !== bytes.length || bytes.length === 0 || bytes.length > resource.size - offset) throw new Error("Extension resource response is inconsistent");
      chunks.push(bytes);
      offset += bytes.length;
      if (chunk.eof !== (offset === resource.size)) throw new Error("Extension resource EOF marker is inconsistent");
    }
    const bytes = joinBytes(chunks, resource.size);
    await verifyExtensionResourceDigest(bytes, resource.sha256);
    return bytes;
  } finally {
    await resourceApi.release({ resourceId: resource.resourceId });
  }
}

function decodeBase64(result: { readonly dataBase64: string }): Uint8Array {
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
