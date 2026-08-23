import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import { USER_THEME_FILES_LIST_CHANNEL, USER_THEME_FILE_DELETE_CHANNEL, USER_THEME_FILE_WRITE_CHANNEL, type IUserThemeFilesApi } from "../common/userThemeFiles.js";

export function createUserThemeFilesApi(): IUserThemeFilesApi {
	return {
		delete: (request) => invoke(USER_THEME_FILE_DELETE_CHANNEL, request),
		list: () => invoke(USER_THEME_FILES_LIST_CHANNEL),
		write: (request) => invoke(USER_THEME_FILE_WRITE_CHANNEL, request),
	};
}
