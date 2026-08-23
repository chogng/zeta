import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import { type IAppServerApi } from "../../app-server/common/appServerApi.js";
import { type ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import { type IDebugAdapterProcessReadResult, type IDebugAdapterProcessService, type IDebugAdapterProcessStartOptions } from "../common/debugAdapterProcessService.js";
import { type RendererHostCapabilities } from "../../renderer/common/rendererHost.js";

/** Vite development adapter for App Server-owned DAP processes. */
export class ViteDevDebugAdapterProcessService implements IDebugAdapterProcessService {
	constructor(private readonly connection: ViteDevAppServerConnection, private readonly appServer: IAppServerApi) {}

	async start(options: IDebugAdapterProcessStartOptions): Promise<string> {
		return (await viteDevRequest(this.connection, "debug/adapter/start", { program: options.program, arguments: [...options.arguments] })).sessionId;
	}

	send(sessionId: string, message: unknown): Promise<void> {
		return voidResult(viteDevRequest(this.connection, "debug/adapter/send", { sessionId, message }));
	}

	read(sessionId: string, afterSequence: number, maxMessages: number): Promise<IDebugAdapterProcessReadResult> {
		return viteDevRequest(this.connection, "debug/adapter/read", { sessionId, afterSequence, maxMessages });
	}

	close(sessionId: string): Promise<void> {
		return voidResult(viteDevRequest(this.connection, "debug/adapter/close", { sessionId }));
	}

	getConnectionState() { return this.appServer.getConnectionState(); }

	onConnectionState(listener: Parameters<IAppServerApi["onConnectionState"]>[0]): IDisposable {
		const subscription = this.appServer.onConnectionState(listener);
		return toDisposable(() => subscription.dispose());
	}
}

/** Code product contribution for the connected Vite renderer host. */
export function createViteDevDebugAdapterCapability(connection: ViteDevAppServerConnection, appServer: IAppServerApi): RendererHostCapabilities {
	return { debugAdapter: new ViteDevDebugAdapterProcessService(connection, appServer) };
}
