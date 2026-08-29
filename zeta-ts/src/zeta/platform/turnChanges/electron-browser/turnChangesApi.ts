import type {
	TurnChangesListResult,
	TurnChangesMutationResult,
	TurnChangesReadFileResult,
	TurnChangesReadResult,
} from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ITurnChangesApi } from "../common/turnChangesApi.js";

export function createTurnChangesApi(): ITurnChangesApi {
	return {
		list: (params) => invoke<TurnChangesListResult>("zeta:turn-changes:list", params),
		read: (params) => invoke<TurnChangesReadResult>("zeta:turn-changes:read", params),
		readFile: (params) => invoke<TurnChangesReadFileResult>("zeta:turn-changes:read-file", params),
		generateMessage: (params) => invoke<TurnChangesMutationResult>("zeta:turn-changes:generate-message", params),
		updateDraft: (params) => invoke<TurnChangesMutationResult>("zeta:turn-changes:update-draft", params),
		commit: (params) => invoke<TurnChangesMutationResult>("zeta:turn-changes:commit", params),
		discardThread: (params) => invoke<TurnChangesMutationResult>("zeta:turn-changes:discard-thread", params),
	};
}
