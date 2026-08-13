import { operatingSystemFromNodePlatform } from "../../../base/common/environment.js";
import { sandboxProcess } from "../../../base/parts/sandbox/electron-browser/globals.js";
import { createAppServerApi, createResourceApi, createServerEventApi } from "../../app-server/electron-browser/appServerApi.js";
import { createBrowserViewApi } from "../../browser/electron-browser/browserViewApi.js";
import { createConfigurationApi } from "../../configuration/electron-browser/configurationApi.js";
import { createElectronExtensionApi } from "../../extensions/electron-browser/extensionApi.js";
import { createNativeContextMenuApi } from "../../contextview/electron-browser/contextMenuApi.js";
import { createFileApi } from "../../files/electron-browser/fileApi.js";
import { createDiffApi } from "../../diff/electron-browser/diffApi.js";
import { createSyntaxApi } from "../../syntax/electron-browser/syntaxApi.js";
import { createDocumentCollaborationApi } from "../../collaboration/electron-browser/documentCollaborationApi.js";
import { createGitApi } from "../../git/electron-browser/gitApi.js";
import { createKeybindingsResourceApi } from "../../keybinding/electron-browser/keybindingsResourceApi.js";
import { createNativeMenubarApi } from "../../menubar/electron-browser/nativeMenubarApi.js";
import { createWorkspaceSearchApi } from "../../search/electron-browser/searchApi.js";
import { createModelApi, createSessionApi, createThreadApi, createTurnApi } from "../../sessions/electron-browser/sessionApi.js";
import { createSkillApi } from "../../skills/electron-browser/skillApi.js";
import { ElectronTerminalProcessService } from "../../terminal/electron-browser/electronTerminalProcessService.js";
import { createUserThemeFilesApi } from "../../theme/electron-browser/userThemeFilesApi.js";
import { createTypstApi } from "../../typst/electron-browser/typstApi.js";
import { createWorkspaceContextApi } from "../../workspace/electron-browser/workspaceContextApi.js";
import { createCodeIndexApi } from "../../codeIndex/electron-browser/codeIndexApi.js";
import { createConnectorApi } from "../../connectors/electron-browser/connectorApi.js";
import { createToolSearchApi } from "../../toolSearch/electron-browser/toolSearchApi.js";
import type { ZetaElectronRendererApi } from "../common/rendererApi.js";
import { createNativeHostApi } from "./nativeHostApi.js";
import { createLanguageApi } from "../../language/electron-browser/languageApi.js";
import { createPluginApi } from "../../plugins/electron-browser/pluginApi.js";

/** Composes Electron renderer capabilities from domain-owned IPC adapters. */
export function createElectronRendererApi(): ZetaElectronRendererApi {
  const appServer = createAppServerApi();
  const resource = createResourceApi();
  return {
    environment: {
      runtime: "electron",
      os: operatingSystemFromNodePlatform(sandboxProcess.platform),
      arch: sandboxProcess.arch,
    },
    appServer,
    browserView: createBrowserViewApi(),
    session: createSessionApi(),
    model: createModelApi(),
    thread: createThreadApi(),
    turn: createTurnApi(),
    skills: createSkillApi(),
    typst: createTypstApi(),
    documentCollaboration: createDocumentCollaborationApi(),
    resource,
    extensions: createElectronExtensionApi(resource),
    fs: createFileApi(),
    diff: createDiffApi(),
    syntax: createSyntaxApi(),
    language: createLanguageApi(),
    git: createGitApi(),
    workspaceSearch: createWorkspaceSearchApi(),
    terminal: new ElectronTerminalProcessService(appServer),
    events: createServerEventApi(),
    configuration: createConfigurationApi(),
    keybindings: createKeybindingsResourceApi(),
    nativeContextMenu: createNativeContextMenuApi(),
    nativeHost: createNativeHostApi(),
    nativeMenubar: createNativeMenubarApi(),
    userThemes: createUserThemeFilesApi(),
    workspace: createWorkspaceContextApi(),
    codeIndex: createCodeIndexApi(),
    connectors: createConnectorApi(),
    plugins: createPluginApi(),
    toolSearch: createToolSearchApi(),
  };
}
