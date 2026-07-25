import { contextBridge, ipcRenderer } from "electron";
import type { ServerNotification } from "../../generated/app-server/v1/types.js";
import type { ZetaPreloadApi } from "./api.js";

const api: ZetaPreloadApi = {
  thread: { start: (params) => ipcRenderer.invoke("zeta:thread:start", params), read: (params) => ipcRenderer.invoke("zeta:thread:read", params) },
  turn: { start: (params) => ipcRenderer.invoke("zeta:turn:start", params), interrupt: (params) => ipcRenderer.invoke("zeta:turn:interrupt", params) },
  events: {
    subscribe: (listener) => {
      const handler = (_event: Electron.IpcRendererEvent, notification: ServerNotification) => listener(notification);
      ipcRenderer.on("zeta:event", handler);
      return () => ipcRenderer.removeListener("zeta:event", handler);
    },
  },
};
contextBridge.exposeInMainWorld("zeta", api);
