import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  NATIVE_HOST_OPEN_FOLDER_CHANNEL,
  NATIVE_HOST_SET_WINDOW_THEME_CHANNEL,
  NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
  type INativeWindowTheme,
  validateNativeWindowTheme,
  validateOpenFolder,
  validateToggleDeveloperTools,
} from "../common/nativeHost.js";

/** Main-process implementation of native operations for one window. */
export interface INativeHostMainService {
  openFolder(): Promise<void>;
  setWindowTheme(theme: INativeWindowTheme): void;
  toggleDeveloperTools(): void;
}

/** Exposes one window's native operations through the trusted IPC router. */
export function nativeHostIpcRoutes(
  service: INativeHostMainService,
): readonly IpcRoute<unknown, unknown>[] {
  return [
    {
      channel: NATIVE_HOST_OPEN_FOLDER_CHANNEL,
      validate: validateOpenFolder,
      invoke: () => service.openFolder(),
    },
    {
      channel: NATIVE_HOST_SET_WINDOW_THEME_CHANNEL,
      validate: validateNativeWindowTheme,
      invoke: (theme) => service.setWindowTheme(theme as INativeWindowTheme),
    },
    {
      channel: NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
      validate: validateToggleDeveloperTools,
      invoke: () => service.toggleDeveloperTools(),
    },
  ];
}
