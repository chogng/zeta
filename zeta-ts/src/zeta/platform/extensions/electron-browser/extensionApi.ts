import type { ExtensionCatalogReload, ExtensionResourceRequest, IExtensionApi } from "../common/extensionApi.js";
import { normalizeExtensionCatalog, normalizeExtensionResourceChunk, normalizeExtensionResourceOpenResult, verifyExtensionResourceDigest } from "../common/extensionApi.js";
import type { ExtensionListResult } from "../../../../../generated/app-server/types.js";
import { decodeBase64, VSBuffer } from "../../../base/common/buffer.js";
import type { IResourceApi } from "../../app-server/common/appServerApi.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";

export function createElectronExtensionApi(resourceApi: IResourceApi): IExtensionApi {
	return {
		list: async (reload: ExtensionCatalogReload) => normalizeExtensionCatalog(await invoke<ExtensionListResult>("zeta:extensions:list", { reload })),
		readResource: request => readExtensionResource(resourceApi, request),
	};
}

async function readExtensionResource(resourceApi: IResourceApi, request: ExtensionResourceRequest): Promise<Uint8Array> {
	const resource = normalizeExtensionResourceOpenResult(await invoke<unknown>("zeta:extensions:resource-open", request));
	const chunks: VSBuffer[] = [];
	let offset = 0;
	try {
		while (offset < resource.size) {
			const chunk = normalizeExtensionResourceChunk(await resourceApi.read({ resourceId: resource.resourceId, offset, maxBytes: Math.min(262_144, resource.size - offset) }));
			const bytes = decodeBase64(chunk.dataBase64);
			if (chunk.resourceId !== resource.resourceId || chunk.offset !== offset || chunk.decodedLength !== bytes.byteLength || bytes.byteLength === 0 || bytes.byteLength > resource.size - offset) throw new Error("Extension resource response is inconsistent");
			chunks.push(bytes);
			offset += bytes.byteLength;
			if (chunk.eof !== (offset === resource.size)) throw new Error("Extension resource EOF marker is inconsistent");
		}
		const bytes = VSBuffer.concat(chunks);
		if (bytes.byteLength !== resource.size) throw new Error("Extension resource byte count is inconsistent");
		await verifyExtensionResourceDigest(bytes.buffer, resource.sha256);
		return bytes.buffer;
	} finally {
		await resourceApi.release({ resourceId: resource.resourceId });
	}
}
