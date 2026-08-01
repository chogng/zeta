import type { ResourceMetadataResult, ResourceReadResult, ServerNotification, SlashCommandDefinition } from "../../../../../generated/app-server/types.js";
import type { AppServerConnectionState, IAppServerApi, IResourceApi, IServerEventApi } from "../common/appServerApi.js";
import { invoke, subscribe } from "../../ipc/electron-browser/rendererIpc.js";

export function createAppServerApi(): IAppServerApi {
  return {
    getConnectionState: () => invoke<AppServerConnectionState>("zeta:app-server:state"),
    getSlashCommands: () => invoke<readonly SlashCommandDefinition[]>("zeta:app-server:slash-commands"),
    onConnectionState: (listener) => subscribe("zeta:app-server:stateChanged", listener),
  };
}

export function createResourceApi(): IResourceApi {
  return {
    metadata: (params) => invoke<ResourceMetadataResult>("zeta:resource:metadata", params),
    read: (params) => invoke<ResourceReadResult>("zeta:resource:read", params),
    release: (params) => invoke<void>("zeta:resource:release", params),
  };
}

export function createServerEventApi(): IServerEventApi {
  return {
    subscribe: (listener) => subscribe<ServerNotification>("zeta:event", listener),
  };
}
