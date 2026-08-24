import { type DebugAdapterCloseParams, type DebugAdapterReadResult, type DebugAdapterSendParams, type DebugAdapterStartParams, type DebugAdapterStartResult } from "../../../../../generated/app-server/types.js";
import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import { type IAppServerApi } from "../../app-server/common/appServerApi.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import { type IDebugAdapterProcessReadResult, type IDebugAdapterProcessService, type IDebugAdapterProcessStartOptions } from "../common/debugAdapterProcessService.js";
import { type RendererHostCapabilities } from "../../renderer/common/rendererHost.js";

/** Electron renderer adapter for App Server-owned DAP processes. */
export class ElectronDebugAdapterProcessService implements IDebugAdapterProcessService {
	private readonly workspaceFolders = new Map<string, string | undefined>();

	constructor(private readonly appServer: IAppServerApi) {}

	async start(options: IDebugAdapterProcessStartOptions): Promise<string> {
		const params: DebugAdapterStartParams = { ...workspaceFolder(options.workspaceFolderId), program: options.program, arguments: [...options.arguments] };
		const sessionId = (await invoke<DebugAdapterStartResult>("zeta:debug-adapter:start", params)).sessionId;
		this.workspaceFolders.set(sessionId, options.workspaceFolderId);
		return sessionId;
	}

	send(sessionId: string, message: unknown): Promise<void> {
		const params: DebugAdapterSendParams = { ...workspaceFolder(this.workspaceFolders.get(sessionId)), sessionId, message };
		return invoke<void>("zeta:debug-adapter:send", params);
	}

	read(sessionId: string, afterSequence: number, maxMessages: number): Promise<IDebugAdapterProcessReadResult> {
		return invoke<DebugAdapterReadResult>("zeta:debug-adapter:read", { ...workspaceFolder(this.workspaceFolders.get(sessionId)), sessionId, afterSequence, maxMessages });
	}

	close(sessionId: string): Promise<void> {
		const params: DebugAdapterCloseParams = { ...workspaceFolder(this.workspaceFolders.get(sessionId)), sessionId };
		return invoke<void>("zeta:debug-adapter:close", params).finally(() => this.workspaceFolders.delete(sessionId));
	}

	getConnectionState() { return this.appServer.getConnectionState(); }

	onConnectionState(listener: Parameters<IAppServerApi["onConnectionState"]>[0]): IDisposable {
		const subscription = this.appServer.onConnectionState(listener);
		return toDisposable(() => subscription.dispose());
	}
}

function workspaceFolder(workspaceFolderId: string | undefined): { readonly workspaceFolderId?: string } {
	return workspaceFolderId === undefined ? {} : { workspaceFolderId };
}

/** Code product contribution for the Electron renderer host. */
export function createElectronDebugAdapterCapability(appServer: IAppServerApi): RendererHostCapabilities {
	return { debugAdapter: new ElectronDebugAdapterProcessService(appServer) };
}
