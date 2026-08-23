import type { DocumentCollaborationOpenResult } from "../../../../../generated/app-server/types.js";
import type { DocumentCollaborationSubmitResult } from "../../../../../generated/app-server/types.js";
import type { DocumentCollaborationPresenceSnapshot } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IDocumentCollaborationApi } from "../common/documentCollaborationApi.js";

export function createDocumentCollaborationApi(): IDocumentCollaborationApi {
	return {
		open: params => invoke<DocumentCollaborationOpenResult>("zeta:document:collaboration:open", params),
		submit: params => invoke<DocumentCollaborationSubmitResult>("zeta:document:collaboration:submit", params),
		publishPresence: params => invoke<DocumentCollaborationPresenceSnapshot>("zeta:document:collaboration:presence:publish", params),
		readPresence: params => invoke<DocumentCollaborationPresenceSnapshot>("zeta:document:collaboration:presence:read", params),
	};
}
