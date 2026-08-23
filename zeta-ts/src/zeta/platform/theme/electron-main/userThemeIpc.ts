import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { USER_THEME_FILES_LIST_CHANNEL, USER_THEME_FILE_DELETE_CHANNEL, USER_THEME_FILE_WRITE_CHANNEL, validateUserThemeFileDeleteRequest, validateUserThemeFileWriteRequest } from "../common/userThemeFiles.js";
import type { UserThemeFileService } from "../node/userThemeFileService.js";

export function userThemeIpcRoutes(service: UserThemeFileService): readonly IpcRoute<unknown, unknown>[] {
	return [
		{
			channel: USER_THEME_FILES_LIST_CHANNEL,
			validate: validateListUserThemes,
			invoke: () => service.list(),
		},
		{
			channel: USER_THEME_FILE_WRITE_CHANNEL,
			validate: validateUserThemeFileWriteRequest,
			invoke: (request) => service.write(validateUserThemeFileWriteRequest(request)),
		},
		{
			channel: USER_THEME_FILE_DELETE_CHANNEL,
			validate: validateUserThemeFileDeleteRequest,
			invoke: (request) => service.delete(validateUserThemeFileDeleteRequest(request)),
		},
	];
}

function validateListUserThemes(value: unknown): undefined {
	if (value !== undefined) throw new Error("list user themes does not accept parameters");
	return undefined;
}
