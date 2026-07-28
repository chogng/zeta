import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron";
import type { ServerNotification } from "../../../generated/app-server/types.js";
import {
  operatingSystemFromNodePlatform,
} from "../../base/common/environment.js";
import type {
  AppServerConnectionState,
} from "../../platform/app-server/common/renderer-api.js";
import type {
  ZetaElectronRendererApi,
} from "../../platform/native/common/rendererApi.js";
import {
  CONFIGURATION_CHANGED_CHANNEL,
  CONFIGURATION_READ_CHANNEL,
  CONFIGURATION_UPDATE_CHANNEL,
} from "../../platform/configuration/common/configuration.js";
import {
  KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
  KEYBINDINGS_RESOURCE_READ_CHANNEL,
  KEYBINDINGS_RESOURCE_UPDATE_CHANNEL,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
  NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
} from "../../base/parts/contextmenu/common/contextmenu.js";
import type {
  INativeMenubarSelection,
} from "../../platform/menubar/common/nativeMenubar.js";
import {
  NATIVE_MENUBAR_SELECT_CHANNEL,
  NATIVE_MENUBAR_UPDATE_CHANNEL,
} from "../../platform/menubar/common/nativeMenubar.js";
import {
  WORKSPACE_CONTEXT_READ_CHANNEL,
} from "../../platform/workspace/common/workspaceIpc.js";

const api: ZetaElectronRendererApi = {
  environment: {
    runtime: "electron",
    os: operatingSystemFromNodePlatform(process.platform),
    arch: process.arch,
  },
  appServer: {
    getConnectionState: () => ipcRenderer.invoke("zeta:app-server:state"),
    onConnectionState: (listener) => {
      const handler = (_event: IpcRendererEvent, state: AppServerConnectionState) =>
        listener(state);
      ipcRenderer.on("zeta:app-server:stateChanged", handler);
      return {
        dispose: () =>
          ipcRenderer.removeListener("zeta:app-server:stateChanged", handler),
      };
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
      return {
        dispose: () => ipcRenderer.removeListener("zeta:event", handler),
      };
    },
  },
  configuration: {
    read: () => ipcRenderer.invoke(CONFIGURATION_READ_CHANNEL),
    update: (request) =>
      ipcRenderer.invoke(CONFIGURATION_UPDATE_CHANNEL, request),
    onDidChange: (listener) => {
      const handler = (
        _event: IpcRendererEvent,
        snapshot: unknown,
      ) => listener(snapshot);
      ipcRenderer.on(CONFIGURATION_CHANGED_CHANNEL, handler);
      return {
        dispose: () =>
          ipcRenderer.removeListener(
            CONFIGURATION_CHANGED_CHANNEL,
            handler,
          ),
      };
    },
  },
  keybindings: {
    read: () => ipcRenderer.invoke(KEYBINDINGS_RESOURCE_READ_CHANNEL),
    update: (request) =>
      ipcRenderer.invoke(KEYBINDINGS_RESOURCE_UPDATE_CHANNEL, request),
    onDidChange: (listener) => {
      const handler = (
        _event: IpcRendererEvent,
        snapshot: unknown,
      ) => listener(snapshot);
      ipcRenderer.on(KEYBINDINGS_RESOURCE_CHANGED_CHANNEL, handler);
      return {
        dispose: () =>
          ipcRenderer.removeListener(
            KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
            handler,
          ),
      };
    },
  },
  nativeContextMenu: {
    popup: (request) =>
      ipcRenderer.invoke(NATIVE_CONTEXT_MENU_POPUP_CHANNEL, request),
    close: () =>
      ipcRenderer.invoke(NATIVE_CONTEXT_MENU_CLOSE_CHANNEL),
  },
  nativeMenubar: {
    update: (data) =>
      ipcRenderer.invoke(NATIVE_MENUBAR_UPDATE_CHANNEL, data),
    onDidSelect: (listener) => {
      const handler = (
        _event: IpcRendererEvent,
        selection: INativeMenubarSelection,
      ) => listener(selection);
      ipcRenderer.on(NATIVE_MENUBAR_SELECT_CHANNEL, handler);
      return {
        dispose: () =>
          ipcRenderer.removeListener(
            NATIVE_MENUBAR_SELECT_CHANNEL,
            handler,
          ),
      };
    },
  },
  workspace: {
    getWorkspace: () =>
      ipcRenderer.invoke(WORKSPACE_CONTEXT_READ_CHANNEL),
  },
};

contextBridge.exposeInMainWorld("zeta", api);
