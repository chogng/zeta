import { createViteDevAppServerApi, createViteDevResourceApi, createViteDevServerEventApi } from "./appServerApi.js";
import { ViteDevAppServerConnection, type ViteDevAppServerConnectionOptions, type ViteDevAppServerMetadata, type ViteDevHotContext } from "./viteDevConnection.js";
import { createViteDevFileApi } from "../../files/browser/fileApi.js";
import { createViteDevExtensionApi } from "../../extensions/browser/extensionApi.js";
import { createViteDevDiffApi } from "../../diff/browser/diffApi.js";
import { createViteDevSyntaxApi } from "../../syntax/browser/syntaxApi.js";
import { createViteDevGitApi } from "../../git/browser/gitApi.js";
import { mergeRendererHostCapabilities, type IRendererHost, type RendererHostCapabilities } from "../../renderer/common/rendererHost.js";
import { createViteDevWorkspaceSearchApi } from "../../search/browser/searchApi.js";
import { createViteDevModelApi, createViteDevSessionApi, createViteDevThreadApi, createViteDevTurnApi } from "../../sessions/browser/sessionApi.js";
import { createViteDevSkillApi } from "../../skills/browser/skillApi.js";
import { ViteDevTerminalProcessService } from "../../terminal/browser/viteDevTerminalProcessService.js";
import { createViteDevTypstApi } from "../../typst/browser/typstApi.js";
import { createViteDevDocumentCollaborationApi } from "../../collaboration/browser/documentCollaborationApi.js";
import { createViteDevCodeIndexApi } from "../../codeIndex/browser/codeIndexApi.js";
import { createViteDevSymbolIndexApi } from "../../symbolIndex/browser/symbolIndexApi.js";
import { createViteDevConnectorApi, type BrowserConnectorHostServices } from "../../connectors/browser/connectorApi.js";
import { createViteDevToolSearchApi } from "../../toolSearch/browser/toolSearchApi.js";
import { createViteDevLanguageApi } from "../../language/browser/languageApi.js";
import { createViteDevPluginApi } from "../../plugins/browser/pluginApi.js";
import { createViteDevExtensionHostApi } from "../../extensionHost/browser/extensionHostApi.js";
import { createViteDevMarketplaceApi } from "../../marketplace/browser/marketplaceApi.js";
import { createViteDevWorkspaceTrustApi } from "../../workspaceTrust/browser/workspaceTrustApi.js";
import { createViteDevAccountApi } from "../../accounts/browser/accountApi.js";
import { createViteDevTurnChangesApi } from "../../turnChanges/browser/turnChangesApi.js";

export type ViteDevRendererCapabilityContribution = (connection: ViteDevAppServerConnection, appServer: IRendererHost["appServer"]) => RendererHostCapabilities;

export interface ConnectedWebRendererApi {
	readonly api: IRendererHost;
	readonly metadata: ViteDevAppServerMetadata;
	dispose(): void;
}

/** Connects a browser Renderer host to the loopback Vite development bridge. */
export async function connectViteDevRendererApi(hot: ViteDevHotContext, connectorHostServices: BrowserConnectorHostServices, options: ViteDevAppServerConnectionOptions = {}, contributions: readonly ViteDevRendererCapabilityContribution[] = []): Promise<ConnectedWebRendererApi> {
	const connection = new ViteDevAppServerConnection(hot, options);
	try {
		const metadata = await connection.connect();
		return {
			api: createRendererHost(connection, connectorHostServices, contributions),
			metadata,
			dispose: () => connection.dispose(),
		};
	} catch (error) {
		connection.dispose();
		throw error;
	}
}

function createRendererHost(connection: ViteDevAppServerConnection, connectorHostServices: BrowserConnectorHostServices, contributions: readonly ViteDevRendererCapabilityContribution[]): IRendererHost {
	const appServer = createViteDevAppServerApi(connection);
	const resource = createViteDevResourceApi(connection);
	const capabilities = mergeRendererHostCapabilities(contributions.map(contribution => contribution(connection, appServer)));
	return {
		appServer,
		accounts: createViteDevAccountApi(connection, connectorHostServices),
		session: createViteDevSessionApi(connection),
		model: createViteDevModelApi(connection),
		thread: createViteDevThreadApi(connection),
		turn: createViteDevTurnApi(connection),
		turnChanges: createViteDevTurnChangesApi(connection),
		skills: createViteDevSkillApi(connection),
		typst: createViteDevTypstApi(connection),
		documentCollaboration: createViteDevDocumentCollaborationApi(connection),
		resource,
		extensions: createViteDevExtensionApi(connection, resource),
		extensionHost: createViteDevExtensionHostApi(connection),
		fs: createViteDevFileApi(connection),
		diff: createViteDevDiffApi(connection),
		syntax: createViteDevSyntaxApi(connection),
		language: createViteDevLanguageApi(connection),
		git: createViteDevGitApi(connection),
		workspaceSearch: createViteDevWorkspaceSearchApi(connection),
		terminal: new ViteDevTerminalProcessService(connection, appServer),
		...capabilities,
		events: createViteDevServerEventApi(connection),
		codeIndex: createViteDevCodeIndexApi(connection),
		symbolIndex: createViteDevSymbolIndexApi(connection),
		connectors: createViteDevConnectorApi(connection, connectorHostServices),
		plugins: createViteDevPluginApi(connection),
		marketplace: createViteDevMarketplaceApi(connection),
		toolSearch: createViteDevToolSearchApi(connection),
		workspaceTrust: createViteDevWorkspaceTrustApi(connection),
	};
}
