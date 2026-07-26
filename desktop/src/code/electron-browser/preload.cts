import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron";
import type { ServerNotification } from "../../../generated/app-server/types.js";
import type {
  AppServerConnectionState,
  ZetaRendererApi,
} from "../../platform/app-server/common/renderer-api.js";

const api: ZetaRendererApi = {
  appServer: {
    getConnectionState: () => ipcRenderer.invoke("zeta:app-server:state"),
    onConnectionState: (listener) => {
      const handler = (_event: IpcRendererEvent, state: AppServerConnectionState) =>
        listener(state);
      ipcRenderer.on("zeta:app-server:stateChanged", handler);
      return () => ipcRenderer.removeListener("zeta:app-server:stateChanged", handler);
    },
  },
  session: {
    create: (params) => ipcRenderer.invoke("zeta:session:create", params),
    read: (params) => ipcRenderer.invoke("zeta:session:read", params),
    list: () => ipcRenderer.invoke("zeta:session:list"),
    subscribe: (params) => ipcRenderer.invoke("zeta:session:subscribe", params),
    unsubscribe: (params) => ipcRenderer.invoke("zeta:session:unsubscribe", params),
    createThread: (params) => ipcRenderer.invoke("zeta:session:thread:create", params),
    forkThread: (params) => ipcRenderer.invoke("zeta:session:thread:fork", params),
    archiveThread: (params) => ipcRenderer.invoke("zeta:session:thread:archive", params),
    complete: (params) => ipcRenderer.invoke("zeta:session:complete", params),
    archive: (params) => ipcRenderer.invoke("zeta:session:archive", params),
  },
  thread: {
    read: (params) => ipcRenderer.invoke("zeta:thread:read", params),
    subscribe: (params) => ipcRenderer.invoke("zeta:thread:subscribe", params),
    unsubscribe: (params) => ipcRenderer.invoke("zeta:thread:unsubscribe", params),
  },
  turn: {
    start: (params) => ipcRenderer.invoke("zeta:turn:start", params),
    interrupt: (params) => ipcRenderer.invoke("zeta:turn:interrupt", params),
  },
  events: {
    subscribe: (listener) => {
      const handler = (_event: IpcRendererEvent, notification: ServerNotification) => listener(notification);
      ipcRenderer.on("zeta:event", handler);
      return () => ipcRenderer.removeListener("zeta:event", handler);
    },
  },
};

contextBridge.exposeInMainWorld("zeta", api);
