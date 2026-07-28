import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
  validateToggleDeveloperTools,
} from "../common/nativeHost.js";

/** Main-process implementation of native operations for one window. */
export interface INativeHostMainService {
  toggleDeveloperTools(): void;
}

/** Exposes one window's native operations through the trusted IPC router. */
export function nativeHostIpcRoutes(
  service: INativeHostMainService,
): readonly IpcRoute<unknown, unknown>[] {
  return [{
    channel: NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
    validate: validateToggleDeveloperTools,
    invoke: () => service.toggleDeveloperTools(),
  }];
}
