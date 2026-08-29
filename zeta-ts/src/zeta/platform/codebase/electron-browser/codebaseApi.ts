import type { CodebaseStatusResult, ConfigCommandResult, ConfigReadResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ICodebaseApi } from "../common/codebaseApi.js";

export function createCodebaseApi(): ICodebaseApi {
	return {
		readConfig: () => invoke<ConfigReadResult>("zeta:codebase:config-read"),
		configureProvider: params => invoke<ConfigCommandResult>("zeta:codebase:provider-configure", params),
		configure: params => invoke<ConfigCommandResult>("zeta:codebase:configure", params),
		status: () => invoke<CodebaseStatusResult>("zeta:codebase:status"),
	};
}
