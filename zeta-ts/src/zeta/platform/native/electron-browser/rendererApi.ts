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
import { createNativeKeyboardLayoutApi } from "../../keyboardLayout/electron-browser/nativeKeyboardLayoutApi.js";
import { createUserKeyboardLayoutApi } from "../../keyboardLayout/electron-browser/userKeyboardLayoutApi.js";
import { createNativeMenubarApi } from "../../menubar/electron-browser/nativeMenubarApi.js";
import { createWorkspaceSearchApi } from "../../search/electron-browser/searchApi.js";
import { createModelApi, createSessionApi, createThreadApi, createTurnApi } from "../../sessions/electron-browser/sessionApi.js";
import { createSkillApi } from "../../skills/electron-browser/skillApi.js";
import { ElectronTerminalProcessService } from "../../terminal/electron-browser/electronTerminalProcessService.js";
import { createUserThemeFilesApi } from "../../theme/electron-browser/userThemeFilesApi.js";
import { createTypstApi } from "../../typst/electron-browser/typstApi.js";
import { createWorkspaceContextApi } from "../../workspace/electron-browser/workspaceContextApi.js";
import { createCodeIndexApi } from "../../codeIndex/electron-browser/codeIndexApi.js";
import { createSymbolIndexApi } from "../../symbolIndex/electron-browser/symbolIndexApi.js";
import { createConnectorApi } from "../../connectors/electron-browser/connectorApi.js";
import { createToolSearchApi } from "../../toolSearch/electron-browser/toolSearchApi.js";
import type { ZetaElectronRendererApi } from "../common/rendererApi.js";
import { createNativeHostApi } from "./nativeHostApi.js";
import { createLanguageApi } from "../../language/electron-browser/languageApi.js";
import { createPluginApi } from "../../plugins/electron-browser/pluginApi.js";
import type { IAppServerApi } from "../../app-server/common/appServerApi.js";
import { mergeRendererHostCapabilities } from "../../renderer/common/rendererHost.js";
import type { RendererHostCapabilities } from "../../renderer/common/rendererHost.js";
import { createElectronExtensionHostApi } from "../../extensionHost/electron-browser/extensionHostApi.js";
import { createRemoteAgentApi } from "../../remote/electron-browser/remoteAgentApi.js";
import { createRemoteConnectionApi } from "../../remote/electron-browser/remoteConnectionApi.js";
import { createRemoteTunnelApi } from "../../remote/electron-browser/remoteTunnelApi.js";
import { createMarketplaceApi } from "../../marketplace/electron-browser/marketplaceApi.js";
import { createWorkspaceTrustApi } from "../../workspaceTrust/electron-browser/workspaceTrustApi.js";
import { createAccountApi } from "../../accounts/electron-browser/accountApi.js";
import { createTurnChangesApi } from "../../turnChanges/electron-browser/turnChangesApi.js";

export type ElectronRendererCapabilityContribution = (appServer: IAppServerApi) => RendererHostCapabilities;

/** Composes Electron renderer capabilities from domain-owned IPC adapters. */
export function createElectronRendererApi(contributions: readonly ElectronRendererCapabilityContribution[] = []): ZetaElectronRendererApi {
	const appServer = createAppServerApi();
	const resource = createResourceApi();
	const capabilities = mergeRendererHostCapabilities(contributions.map(contribution => contribution(appServer)));
	return {
		environment: {
			runtime: "electron",
			os: operatingSystemFromNodePlatform(sandboxProcess.platform),
			arch: sandboxProcess.arch,
		},
		appServer,
		accounts: createAccountApi(),
		remote: createRemoteAgentApi(),
		remoteConnections: createRemoteConnectionApi(),
		remoteTunnels: createRemoteTunnelApi(),
		browserView: createBrowserViewApi(),
		session: createSessionApi(),
		model: createModelApi(),
		thread: createThreadApi(),
		turn: createTurnApi(),
		turnChanges: createTurnChangesApi(),
		skills: createSkillApi(),
		typst: createTypstApi(),
		documentCollaboration: createDocumentCollaborationApi(),
		resource,
		extensions: createElectronExtensionApi(resource),
		extensionHost: createElectronExtensionHostApi(),
		fs: createFileApi(),
		diff: createDiffApi(),
		syntax: createSyntaxApi(),
		language: createLanguageApi(),
		git: createGitApi(),
		workspaceSearch: createWorkspaceSearchApi(),
		terminal: new ElectronTerminalProcessService(appServer),
		...capabilities,
		events: createServerEventApi(),
		configuration: createConfigurationApi(),
		keybindings: createKeybindingsResourceApi(),
		keyboardLayout: createNativeKeyboardLayoutApi(),
		userKeyboardLayout: createUserKeyboardLayoutApi(),
		nativeContextMenu: createNativeContextMenuApi(),
		nativeHost: createNativeHostApi(),
		nativeMenubar: createNativeMenubarApi(),
		userThemes: createUserThemeFilesApi(),
		workspace: createWorkspaceContextApi(),
		codeIndex: createCodeIndexApi(),
		symbolIndex: createSymbolIndexApi(),
		connectors: createConnectorApi(),
		plugins: createPluginApi(),
		marketplace: createMarketplaceApi(),
		toolSearch: createToolSearchApi(),
		workspaceTrust: createWorkspaceTrustApi(),
	};
}
