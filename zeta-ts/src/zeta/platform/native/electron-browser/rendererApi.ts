import { AppServerProtocolIncompatibleError } from '../../app-server/common/appServerProtocolCompatibility.js';
import { AppServerProtocolClient } from '../../app-server/browser/appServerProtocolClient.js';
import { AppServerMessagePortTransport } from '../../app-server/electron-browser/appServerMessagePortTransport.js';
import { createRendererHost, type RendererCapabilityContribution } from '../../app-server/browser/webRendererApi.js';
import { createDisconnectedRendererApi } from '../../app-server/browser/rendererApi.js';
import { AppServerAutomationService } from '../../automation/browser/appServerAutomationService.js';
import { registerAppServerBrowserHost } from '../../browser/electron-browser/appServerBrowserHost.js';
import { registerAppServerWorkspaceHost, initializeWorkspace } from '../../workspaces/electron-browser/appServerWorkspaceHost.js';
import { DisposableStore, toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';
import type { IRendererHost } from '../../renderer/common/rendererHost.js';
import { invoke } from '../../ipc/electron-browser/rendererIpc.js';
import { subscribe } from '../../ipc/electron-browser/rendererIpc.js';
import { ReconnectableTerminalProcessService } from '../../terminal/browser/reconnectableTerminalProcessService.js';
import { operatingSystemFromNodePlatform } from "../../../base/common/environment.js";
import { sandboxProcess } from "../../../base/parts/sandbox/electron-browser/globals.js";
import { createBrowserViewApi } from "../../browser/electron-browser/browserViewApi.js";
import { createConfigurationApi } from "../../configuration/electron-browser/configurationApi.js";
import { createNativeContextMenuApi } from "../../contextview/electron-browser/contextMenuApi.js";
import { createKeybindingsResourceApi } from "../../keybinding/electron-browser/keybindingsResourceApi.js";
import { createNativeKeyboardLayoutApi } from "../../keyboardLayout/electron-browser/nativeKeyboardLayoutApi.js";
import { createUserKeyboardLayoutApi } from "../../keyboardLayout/electron-browser/userKeyboardLayoutApi.js";
import { createNativeMenubarApi } from "../../menubar/electron-browser/nativeMenubarApi.js";
import { createUserThemeFilesApi } from "../../theme/electron-browser/userThemeFilesApi.js";
import { createWorkspaceContextApi } from "../../workspace/electron-browser/workspaceContextApi.js";
import type { ZetaElectronRendererApi } from "../common/rendererApi.js";
import { createNativeHostApi } from "./nativeHostApi.js";
import type { IAppServerApi } from "../../app-server/common/appServerApi.js";
import type { RendererHostCapabilities } from "../../renderer/common/rendererHost.js";
import { createRemoteAgentApi } from "../../remote/electron-browser/remoteAgentApi.js";
import { createRemoteConnectionApi } from "../../remote/electron-browser/remoteConnectionApi.js";
import { createRemoteTunnelApi } from "../../remote/electron-browser/remoteTunnelApi.js";

export type ElectronRendererCapabilityContribution = RendererCapabilityContribution;

/** Composes Electron renderer capabilities from domain-owned IPC adapters. */
export async function createElectronRendererApi(contributions: readonly ElectronRendererCapabilityContribution[] = [], hostCapabilities: { readonly browser: boolean } = { browser: true }): Promise<ZetaElectronRendererApi & IDisposable> {
	const resources = new DisposableStore();
	let connecting: Promise<void> = Promise.resolve();
	const transport = resources.add(new AppServerMessagePortTransport(() => {
		connecting = reconnect();
		void connecting.catch(error => console.error('App Server reconnect failed', error));
	}));
	const client = new AppServerProtocolClient(transport, { clientName: 'zeta-desktop', capabilities: { ...(hostCapabilities.browser ? { browser: { version: 1, observe: true, input: true } } : {}), dirPermissionsHost: { version: 1 } } });
	resources.add(toDisposable(() => client.dispose()));
	if (hostCapabilities.browser) { resources.add(registerAppServerBrowserHost(client)); }
	resources.add(registerAppServerWorkspaceHost(client, () => connecting));
	const initialize = async (): Promise<void> => {
		try { await client.connect(); }
		catch (error) {
			if (!(error instanceof AppServerProtocolIncompatibleError) || await invoke('zeta:app-server:recover-runtime', error.incompatibility) !== true) { throw error; }
			await transport.acquire();
			await client.connect();
		}
		await transport.initialized();
	};
	let reconnectTask: Promise<void> | undefined;
	let attempts = 0;
	let recovering = false;
	let retryTimer: ReturnType<typeof setTimeout> | undefined;
	let started = false;
	resources.add(toDisposable(() => { started = false; if (retryTimer !== undefined) { clearTimeout(retryTimer); } }));
	const reconnect = (): Promise<void> => {
		if (reconnectTask) { return reconnectTask; }
		if (retryTimer !== undefined) { clearTimeout(retryTimer); retryTimer = undefined; }
		const operation = (async () => {
			client.disconnect();
			await transport.acquire();
			await initialize();
		})();
		reconnectTask = operation.finally(() => { reconnectTask = undefined; });
		return reconnectTask;
	};
	const scheduleRecovery = (): void => {
		if (!started || recovering || retryTimer !== undefined || attempts >= 3) { return; }
		retryTimer = setTimeout(() => {
			retryTimer = undefined;
			recovering = true;
			attempts++;
			connecting = reconnect();
			void connecting.then(() => { attempts = 0; }, error => console.error('App Server connection recovery failed', error)).finally(() => {
				recovering = false;
				if (client.state === 'crashed') { scheduleRecovery(); }
			});
		}, [100, 500, 2000][attempts]);
	};
	resources.add(client.onStateChange(state => { if (state === 'crashed') { scheduleRecovery(); } }));
	try {
		const enabled = await transport.acquire();
		let backend: IRendererHost;
		if (enabled) {
			await initialize();
			backend = createRendererHost(client, {
				openerService: { openExternal: target => invoke<void>('zeta:host:openExternal', target) },
				callbackHost: {
					listen: () => invoke('zeta:oauth-callback:listen'),
					wait: id => invoke('zeta:oauth-callback:wait', { id }),
					close: id => invoke('zeta:oauth-callback:close', { id }),
				},
				clipboardService: { readText: () => invoke<string>('zeta:host:readClipboard'), writeText: text => invoke<void>('zeta:host:writeClipboard', text) },
			}, contributions);
			if (client.capabilities?.contracts.automation?.version === 1) { backend = { ...backend, automation: resources.add(new AppServerAutomationService(client)) }; }
			if ((await createRemoteAgentApi().getConnection()).kind === 'ssh') {
				const terminals = resources.add(new ReconnectableTerminalProcessService({ supervisor: client }));
				const ordinary = backend.terminal;
				backend = { ...backend, terminal: {
					listProfiles: () => ordinary.listProfiles(),
					create: options => terminals.create({ ...options, lifecycle: { type: 'connectionOwned' } }), write: options => terminals.write(options),
					resize: options => terminals.resize(options), read: options => terminals.read(options), close: options => terminals.close(options),
					getConnectionState: () => ordinary.getConnectionState(), onConnectionState: listener => ordinary.onConnectionState(listener),
				} };
				const replacement = subscribe('zeta:terminal:prepareReplacement', () => terminals.prepareForServerReplacement());
				resources.add(toDisposable(() => replacement.dispose()));
			}
			await initializeWorkspace(client);
			started = true;
		} else {
			backend = createDisconnectedRendererApi();
		}

		return {
			...backend,
			dispose: () => resources.dispose(),
			[Symbol.dispose]: () => resources.dispose(),
			environment: {
				runtime: "electron",
				os: operatingSystemFromNodePlatform(sandboxProcess.platform),
				arch: sandboxProcess.arch,
			},
			remote: createRemoteAgentApi(),
			remoteConnections: createRemoteConnectionApi(),
			remoteTunnels: createRemoteTunnelApi(),
			browserView: createBrowserViewApi(),
			configuration: createConfigurationApi(),
			keybindings: createKeybindingsResourceApi(),
			keyboardLayout: createNativeKeyboardLayoutApi(),
			userKeyboardLayout: createUserKeyboardLayoutApi(),
			nativeContextMenu: createNativeContextMenuApi(),
			nativeHost: createNativeHostApi(),
			nativeMenubar: createNativeMenubarApi(),
			userThemes: createUserThemeFilesApi(),
			workspace: createWorkspaceContextApi(),
		};
	} catch (error) { resources.dispose(); throw error; }
}
