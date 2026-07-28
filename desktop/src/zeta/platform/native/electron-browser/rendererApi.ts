import type {
  FsGetMetadataResult,
  FsReadDirectoryResult,
  ResourceMetadataResult,
  ResourceReadResult,
  ServerNotification,
  SessionListResult,
  SessionResult,
  SessionSubscribeResult,
  SessionThreadResult,
  ThreadReadResult,
  ThreadSubscribeResult,
  TurnInterruptResult,
  TurnStartResult,
  TypstCompileResult,
} from "../../../../../generated/app-server/types.js";
import {
  operatingSystemFromNodePlatform,
} from "../../../base/common/environment.js";
import {
  ipcRenderer,
  sandboxProcess,
} from "../../../base/parts/sandbox/electron-browser/globals.js";
import {
  NATIVE_CONTEXT_MENU_CLOSE_CHANNEL,
  NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
  type INativeContextMenuResult,
} from "../../../base/parts/contextmenu/common/contextmenu.js";
import {
  type AppServerConnectionState,
} from "../../app-server/common/renderer-api.js";
import {
  BROWSER_VIEW_CLOSE_CHANNEL,
  BROWSER_VIEW_CREATE_CHANNEL,
  BROWSER_VIEW_EVENT_CHANNEL,
  BROWSER_VIEW_GO_BACK_CHANNEL,
  BROWSER_VIEW_GO_FORWARD_CHANNEL,
  BROWSER_VIEW_LAYOUT_CHANNEL,
  BROWSER_VIEW_NAVIGATE_CHANNEL,
  BROWSER_VIEW_RELOAD_CHANNEL,
  BROWSER_VIEW_STATE_CHANNEL,
  BROWSER_VIEW_STOP_CHANNEL,
  BROWSER_VIEW_VISIBILITY_CHANNEL,
  type BrowserViewEvent,
  type IBrowserViewState,
} from "../../browser/common/browserView.js";
import {
  CONFIGURATION_CHANGED_CHANNEL,
  CONFIGURATION_READ_CHANNEL,
  CONFIGURATION_UPDATE_CHANNEL,
} from "../../configuration/common/configuration.js";
import {
  KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
  KEYBINDINGS_RESOURCE_READ_CHANNEL,
  KEYBINDINGS_RESOURCE_UPDATE_CHANNEL,
} from "../../keybinding/common/keybindingsResource.js";
import {
  NATIVE_MENUBAR_SELECT_CHANNEL,
  NATIVE_MENUBAR_UPDATE_CHANNEL,
  type INativeMenubarSelection,
} from "../../menubar/common/nativeMenubar.js";
import {
  NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL,
} from "../common/nativeHost.js";
import {
  WORKSPACE_CONTEXT_READ_CHANNEL,
} from "../../workspace/common/workspaceIpc.js";
import type {
  ZetaElectronRendererApi,
} from "../common/rendererApi.js";

/** Builds Zeta's typed renderer API on top of the minimal sandbox globals. */
export function createElectronRendererApi(): ZetaElectronRendererApi {
  return {
    environment: {
      runtime: "electron",
      os: operatingSystemFromNodePlatform(sandboxProcess.platform),
      arch: sandboxProcess.arch,
    },
    appServer: {
      getConnectionState: () =>
        invoke<AppServerConnectionState>("zeta:app-server:state"),
      onConnectionState: (listener) =>
        subscribe(
          "zeta:app-server:stateChanged",
          listener,
        ),
    },
    browserView: {
      create: (request) =>
        invoke<IBrowserViewState>(BROWSER_VIEW_CREATE_CHANNEL, request),
      getState: (request) =>
        invoke<IBrowserViewState>(BROWSER_VIEW_STATE_CHANNEL, request),
      layout: (request) =>
        invoke<void>(BROWSER_VIEW_LAYOUT_CHANNEL, request),
      setVisibility: (request) =>
        invoke<void>(BROWSER_VIEW_VISIBILITY_CHANNEL, request),
      navigate: (request) =>
        invoke<void>(BROWSER_VIEW_NAVIGATE_CHANNEL, request),
      goBack: (request) =>
        invoke<void>(BROWSER_VIEW_GO_BACK_CHANNEL, request),
      goForward: (request) =>
        invoke<void>(BROWSER_VIEW_GO_FORWARD_CHANNEL, request),
      reload: (request) =>
        invoke<void>(BROWSER_VIEW_RELOAD_CHANNEL, request),
      stop: (request) =>
        invoke<void>(BROWSER_VIEW_STOP_CHANNEL, request),
      close: (request) =>
        invoke<void>(BROWSER_VIEW_CLOSE_CHANNEL, request),
      onDidEvent: (listener) =>
        subscribe<BrowserViewEvent>(BROWSER_VIEW_EVENT_CHANNEL, listener),
    },
    session: {
      create: (params) =>
        invoke<SessionResult>("zeta:session:create", params),
      read: (params) =>
        invoke<SessionResult>("zeta:session:read", params),
      list: () =>
        invoke<SessionListResult>("zeta:session:list"),
      subscribe: (params) =>
        invoke<SessionSubscribeResult>(
          "zeta:session:subscribe",
          params,
        ),
      unsubscribe: (params) =>
        invoke<void>("zeta:session:unsubscribe", params),
      createThread: (params) =>
        invoke<SessionThreadResult>(
          "zeta:session:thread:create",
          params,
        ),
      forkThread: (params) =>
        invoke<SessionThreadResult>(
          "zeta:session:thread:fork",
          params,
        ),
      archiveThread: (params) =>
        invoke<SessionResult>(
          "zeta:session:thread:archive",
          params,
        ),
      complete: (params) =>
        invoke<SessionResult>("zeta:session:complete", params),
      archive: (params) =>
        invoke<SessionResult>("zeta:session:archive", params),
    },
    thread: {
      read: (params) =>
        invoke<ThreadReadResult>("zeta:thread:read", params),
      subscribe: (params) =>
        invoke<ThreadSubscribeResult>(
          "zeta:thread:subscribe",
          params,
        ),
      unsubscribe: (params) =>
        invoke<void>("zeta:thread:unsubscribe", params),
    },
    turn: {
      start: (params) =>
        invoke<TurnStartResult>("zeta:turn:start", params),
      interrupt: (params) =>
        invoke<TurnInterruptResult>("zeta:turn:interrupt", params),
    },
    typst: {
      compile: (params) =>
        invoke<TypstCompileResult>("zeta:typst:compile", params),
    },
    resource: {
      metadata: (params) =>
        invoke<ResourceMetadataResult>("zeta:resource:metadata", params),
      read: (params) =>
        invoke<ResourceReadResult>("zeta:resource:read", params),
      release: (params) =>
        invoke<void>("zeta:resource:release", params),
    },
    fs: {
      getMetadata: (params) =>
        invoke<FsGetMetadataResult>("zeta:fs:get-metadata", params),
      readDirectory: (params) =>
        invoke<FsReadDirectoryResult>("zeta:fs:read-directory", params),
    },
    events: {
      subscribe: (listener) =>
        subscribe<ServerNotification>("zeta:event", listener),
    },
    configuration: {
      read: () => invoke(CONFIGURATION_READ_CHANNEL),
      update: (request) =>
        invoke(CONFIGURATION_UPDATE_CHANNEL, request),
      onDidChange: (listener) =>
        subscribe(CONFIGURATION_CHANGED_CHANNEL, listener),
    },
    keybindings: {
      read: () => invoke(KEYBINDINGS_RESOURCE_READ_CHANNEL),
      update: (request) =>
        invoke(KEYBINDINGS_RESOURCE_UPDATE_CHANNEL, request),
      onDidChange: (listener) =>
        subscribe(KEYBINDINGS_RESOURCE_CHANGED_CHANNEL, listener),
    },
    nativeContextMenu: {
      popup: (request) =>
        invoke<INativeContextMenuResult>(
          NATIVE_CONTEXT_MENU_POPUP_CHANNEL,
          request,
        ),
      close: () =>
        invoke<void>(NATIVE_CONTEXT_MENU_CLOSE_CHANNEL),
    },
    nativeHost: {
      toggleDeveloperTools: () =>
        invoke<void>(NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL),
    },
    nativeMenubar: {
      update: (data) =>
        invoke<void>(NATIVE_MENUBAR_UPDATE_CHANNEL, data),
      onDidSelect: (listener) =>
        subscribe<INativeMenubarSelection>(
          NATIVE_MENUBAR_SELECT_CHANNEL,
          listener,
        ),
    },
    workspace: {
      getWorkspace: () => invoke(WORKSPACE_CONTEXT_READ_CHANNEL),
    },
  };
}

function invoke<TResult>(
  channel: string,
  params?: unknown,
): Promise<TResult> {
  return ipcRenderer.invoke(channel, params) as Promise<TResult>;
}

function subscribe<T>(
  channel: string,
  listener: (value: T) => void,
) {
  return ipcRenderer.on(channel, (value) => listener(value as T));
}
