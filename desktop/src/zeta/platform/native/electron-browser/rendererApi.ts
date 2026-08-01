import { operatingSystemFromNodePlatform } from "../../../base/common/environment.js";
import { sandboxProcess } from "../../../base/parts/sandbox/electron-browser/globals.js";
import { createAppServerApi, createResourceApi, createServerEventApi } from "../../app-server/electron-browser/appServerApi.js";
import { createBrowserViewApi } from "../../browser/electron-browser/browserViewApi.js";
import { createConfigurationApi } from "../../configuration/electron-browser/configurationApi.js";
import { createNativeContextMenuApi } from "../../contextview/electron-browser/contextMenuApi.js";
import { createFileApi } from "../../files/electron-browser/fileApi.js";
import { createGitApi } from "../../git/electron-browser/gitApi.js";
import { createKeybindingsResourceApi } from "../../keybinding/electron-browser/keybindingsResourceApi.js";
import { createNativeMenubarApi } from "../../menubar/electron-browser/nativeMenubarApi.js";
import { createWorkspaceSearchApi } from "../../search/electron-browser/searchApi.js";
import { createModelApi, createSessionApi, createThreadApi, createTurnApi } from "../../sessions/electron-browser/sessionApi.js";
import { createTerminalProcessApi } from "../../terminal/electron-browser/terminalProcessApi.js";
import { createUserThemeFilesApi } from "../../theme/electron-browser/userThemeFilesApi.js";
import { createTypstApi } from "../../typst/electron-browser/typstApi.js";
import { createWorkspaceContextApi } from "../../workspace/electron-browser/workspaceContextApi.js";
import type { ZetaElectronRendererApi } from "../common/rendererApi.js";
import { createNativeHostApi } from "./nativeHostApi.js";

/** Composes Electron renderer capabilities from domain-owned IPC adapters. */
export function createElectronRendererApi(): ZetaElectronRendererApi {
  return {
    environment: {
      runtime: "electron",
      os: operatingSystemFromNodePlatform(sandboxProcess.platform),
      arch: sandboxProcess.arch,
    },
    appServer: createAppServerApi(),
    browserView: createBrowserViewApi(),
    session: createSessionApi(),
    model: createModelApi(),
    thread: createThreadApi(),
    turn: createTurnApi(),
    typst: createTypstApi(),
    resource: createResourceApi(),
    fs: createFileApi(),
    git: createGitApi(),
    workspaceSearch: createWorkspaceSearchApi(),
    terminal: createTerminalProcessApi(),
    events: createServerEventApi(),
    configuration: createConfigurationApi(),
    keybindings: createKeybindingsResourceApi(),
    nativeContextMenu: createNativeContextMenuApi(),
    nativeHost: createNativeHostApi(),
    nativeMenubar: createNativeMenubarApi(),
    userThemes: createUserThemeFilesApi(),
    workspace: createWorkspaceContextApi(),
  };
}
