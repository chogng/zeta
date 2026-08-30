import type { ConfigCommandResult, DirPermissionsForgetParams, DirPermissionsListResult, DirPermissionsReadParams, DirPermissionsReadResult, DirPermissionsSetParams } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IDirPermissionsApi } from "../common/dirPermissionsApi.js";

export function createDirPermissionsApi(): IDirPermissionsApi {
	return {
		list: () => invoke<DirPermissionsListResult>("zeta:dir-permissions:list"),
		read: params => invoke<DirPermissionsReadResult>("zeta:dir-permissions:read", params as DirPermissionsReadParams),
		set: params => invoke<ConfigCommandResult>("zeta:dir-permissions:set", params as DirPermissionsSetParams),
		forget: params => invoke<ConfigCommandResult>("zeta:dir-permissions:forget", params as DirPermissionsForgetParams),
	};
}
