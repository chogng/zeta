import type { PluginCommandResultDto, PluginListResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IPluginApi } from "../common/pluginApi.js";

export function createPluginApi(): IPluginApi {
	return {
		list: () => invoke<PluginListResult>("zeta:plugins:list"),
		enable: params => invoke<PluginCommandResultDto>("zeta:plugins:enable", params),
		disable: params => invoke<PluginCommandResultDto>("zeta:plugins:disable", params),
		grant: params => invoke<PluginCommandResultDto>("zeta:plugins:grant", params),
		revokeGrant: params => invoke<PluginCommandResultDto>("zeta:plugins:revoke-grant", params),
		uninstall: params => invoke<PluginCommandResultDto>("zeta:plugins:uninstall", params),
	};
}
