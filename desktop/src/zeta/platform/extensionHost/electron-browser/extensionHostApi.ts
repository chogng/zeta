import type { IExtensionHostApi } from "../common/extensionHostApi.js";
import { invokeExtensionHost, normalizeExtensionHostChanged, normalizeExtensionHostSnapshot } from "../common/extensionHostApi.js";
import { invoke, subscribe } from "../../ipc/electron-browser/rendererIpc.js";
import type { ServerNotification } from "../../../../../generated/app-server/types.js";
import type { AppServerConnectionState } from "../../app-server/common/appServerApi.js";

export function createElectronExtensionHostApi(): IExtensionHostApi {
  const transport = {
    start: (request: Parameters<IExtensionHostApi["invoke"]>[0]) => invoke<unknown>("zeta:extension-host:invoke-start", request),
    read: (invocationId: string) => invoke<unknown>("zeta:extension-host:invoke-read", { invocationId }),
    cancel: (invocationId: string) => invoke<unknown>("zeta:extension-host:invoke-cancel", { invocationId }),
  };
  return {
    isAvailable: () => invoke<boolean>("zeta:extension-host:available"),
    list: async () => normalizeExtensionHostSnapshot(await invoke<unknown>("zeta:extension-host:list")),
    reconcile: async mode => normalizeExtensionHostSnapshot(await invoke<unknown>("zeta:extension-host:reconcile", { mode })),
    invoke: (request, signal) => invokeExtensionHost(transport, request, signal),
    getConnectionState: () => invoke<AppServerConnectionState>("zeta:app-server:state"),
    onDidChange: listener => subscribe<ServerNotification>("zeta:event", event => {
      if (event.method === "extensionHost/changed") listener(normalizeExtensionHostChanged(event.params));
    }),
    onConnectionState: listener => subscribe<AppServerConnectionState>("zeta:app-server:stateChanged", listener),
  };
}
