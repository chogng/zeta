import type { CodeIndexStatusResult, ConfigCommandResult, ConfigReadResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ICodeIndexApi } from "../common/codeIndexApi.js";

export function createCodeIndexApi(): ICodeIndexApi {
	return {
		readConfig: () => invoke<ConfigReadResult>("zeta:code-index:config-read"),
		configureProvider: params => invoke<ConfigCommandResult>("zeta:code-index:provider-configure", params),
		configure: params => invoke<ConfigCommandResult>("zeta:code-index:semantic-configure", params),
		authorize: params => invoke<ConfigCommandResult>("zeta:code-index:semantic-authorize", params),
		revoke: params => invoke<ConfigCommandResult>("zeta:code-index:semantic-revoke", params),
		status: () => invoke<CodeIndexStatusResult>("zeta:code-index:status"),
		cancel: () => invoke<CodeIndexStatusResult>("zeta:code-index:semantic-cancel"),
		retry: () => invoke<CodeIndexStatusResult>("zeta:code-index:semantic-retry"),
	};
}
