import type { ContentSearchReadResult, ContentSearchStartResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IContentSearchApi } from "../common/searchApi.js";

export function createContentSearchApi(): IContentSearchApi {
	return {
		start: (params) => invoke<ContentSearchStartResult>("zeta:content-search:start", params),
		read: (params) => invoke<ContentSearchReadResult>("zeta:content-search:read", params),
		cancel: (params) => invoke<void>("zeta:content-search:cancel", params),
	};
}
