import { NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL, validateToggleDeveloperTools, } from "../common/nativeHost.js";
/** Exposes one window's native operations through the trusted IPC router. */
export function nativeHostIpcRoutes(service) {
    return [{
            channel: NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
            validate: validateToggleDeveloperTools,
            invoke: () => service.toggleDeveloperTools(),
        }];
}
