import type { ConfigCommandResult, DirPermissionsForgetParams, DirPermissionsListResult, DirPermissionsReadParams, DirPermissionsReadResult, DirPermissionsSetParams } from "../../../../../generated/app-server/types.js";

/** Transport-only directory-permission management operations. */
export interface IDirPermissionsApi {
	list(): Promise<DirPermissionsListResult>;
	read(params: DirPermissionsReadParams): Promise<DirPermissionsReadResult>;
	set(params: DirPermissionsSetParams): Promise<ConfigCommandResult>;
	forget(params: DirPermissionsForgetParams): Promise<ConfigCommandResult>;
}
