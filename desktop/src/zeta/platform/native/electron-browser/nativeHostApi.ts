import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import { NATIVE_HOST_OPEN_FOLDER_CHANNEL, NATIVE_HOST_SET_WINDOW_THEME_CHANNEL, NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL, type INativeHostApi } from "../common/nativeHost.js";

export function createNativeHostApi(): INativeHostApi {
  return {
    openFolder: () => invoke<void>(NATIVE_HOST_OPEN_FOLDER_CHANNEL),
    setWindowTheme: (theme) => invoke<void>(NATIVE_HOST_SET_WINDOW_THEME_CHANNEL, theme),
    toggleDeveloperTools: () => invoke<void>(NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL),
  };
}
