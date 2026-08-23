import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import type { IAppServerApi } from "../../app-server/common/appServerApi.js";
import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { ITerminalProcessCloseOptions, ITerminalProcessCreateOptions, ITerminalProcessCreation, ITerminalProcessProfile, ITerminalProcessReadOptions, ITerminalProcessReadResult, ITerminalProcessResizeOptions, ITerminalProcessService, ITerminalProcessWriteOptions, TerminalProcessConnectionState } from "../common/terminalProcessService.js";

/** Vite development implementation of the terminal process service. */
export class ViteDevTerminalProcessService implements ITerminalProcessService {
	constructor(private readonly connection: ViteDevAppServerConnection, private readonly appServerApi: IAppServerApi) {}

	async listProfiles(): Promise<readonly ITerminalProcessProfile[]> {
		const result = await viteDevRequest(this.connection, "terminal/profile/list", {});
		return result.profiles;
	}

	async create(options: ITerminalProcessCreateOptions): Promise<ITerminalProcessCreation> {
		const created = await viteDevRequest(this.connection, "terminal/create", { ...options, lifecycle: { type: "connectionOwned" } });
		return { terminalId: created.terminalId, profile: created.profile, connectionPersistence: "connectionOwned" };
	}

	write(options: ITerminalProcessWriteOptions): Promise<void> {
		return voidResult(viteDevRequest(this.connection, "terminal/write", options));
	}

	resize(options: ITerminalProcessResizeOptions): Promise<void> {
		return voidResult(viteDevRequest(this.connection, "terminal/resize", options));
	}

	read(options: ITerminalProcessReadOptions): Promise<ITerminalProcessReadResult> {
		return viteDevRequest(this.connection, "terminal/read", options);
	}

	close(options: ITerminalProcessCloseOptions): Promise<void> {
		return voidResult(viteDevRequest(this.connection, "terminal/close", options));
	}

	getConnectionState(): Promise<TerminalProcessConnectionState> {
		return this.appServerApi.getConnectionState();
	}

	onConnectionState(listener: (state: TerminalProcessConnectionState) => void): IDisposable {
		const subscription = this.appServerApi.onConnectionState(listener);
		return toDisposable(() => subscription.dispose());
	}
}
