import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { AppServerConnectionState, IAppServerApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { RemoteConnectionState } from "../../../../platform/remote/common/remote.js";
import type { IRemoteAgentApi, RemoteAgentConnection, RemoteAgentReconnectResult, RemoteRuntimeRollbackResult } from "../../../../platform/remote/common/remoteAgentApi.js";
import type { IRemoteAgentService } from "../common/remoteAgentService.js";

export interface AppServerRemoteAgentServiceOptions {
	readonly api: IAppServerApi;
	readonly remoteApi?: IRemoteAgentApi;
	readonly onReadError?: (error: unknown) => void;
}

/** Adapts the App Server supervisor state into the Workbench remote-agent contract. */
export class AppServerRemoteAgentService extends DisposableOwner implements IRemoteAgentService {
	private readonly connectionStateEmitter = this.own(new Emitter<RemoteConnectionState>());
	private readonly connectionEmitter = this.own(new Emitter<RemoteAgentConnection>());
	private revision = 0;
	private connectionRevision = 0;
	private disposed = false;
	private readonly remoteApi: IRemoteAgentApi | undefined;
	private _connectionState: RemoteConnectionState | undefined;
	private _connection: RemoteAgentConnection | undefined;

	readonly onDidChangeConnectionState = this.connectionStateEmitter.event;
	readonly onDidChangeConnection = this.connectionEmitter.event;

	constructor(options: AppServerRemoteAgentServiceOptions) {
		super();
		this.remoteApi = options.remoteApi;
		const subscription = options.api.onConnectionState(state => {
			if (this.disposed) return;
			this.revision += 1;
			this.acceptState(state);
		});
		this.defer(() => subscription.dispose());
		if (options.remoteApi) this.observeConnection(options.remoteApi);
		const readRevision = this.revision;
		void Promise.resolve()
			.then(() => this.disposed ? undefined : options.api.getConnectionState())
			.then(state => {
				if (!this.disposed && state !== undefined && this.revision === readRevision) this.acceptState(state);
			}, error => {
				if (this.disposed || this.revision !== readRevision) return;
				(options.onReadError ?? defaultReadErrorHandler)(error);
				this.setConnectionState("disconnected");
			});
		this.defer(() => { this.disposed = true; });
	}

	get connectionState(): RemoteConnectionState | undefined {
		return this._connectionState;
	}

	get connection(): RemoteAgentConnection | undefined {
		return this._connection;
	}

	reconnect(): Promise<RemoteAgentReconnectResult> {
		if (!this.remoteApi) return Promise.reject(new Error("Remote reconnect requires a native Remote host"));
		if (this._connection?.kind !== "ssh") return Promise.reject(new Error("Remote reconnect requires an SSH Remote Workspace"));
		return this.remoteApi.reconnect();
	}

	rollbackRuntime(): Promise<RemoteRuntimeRollbackResult> {
		if (!this.remoteApi) return Promise.reject(new Error("Remote runtime rollback requires a native Remote host"));
		if (this._connection?.kind !== "ssh") return Promise.reject(new Error("Remote runtime rollback requires an SSH Remote Workspace"));
		return this.remoteApi.rollbackRuntime();
	}

	private acceptState(state: AppServerConnectionState): void {
		this.setConnectionState(toRemoteConnectionState(state));
	}

	private setConnectionState(state: RemoteConnectionState): void {
		if (this._connectionState === state) return;
		this._connectionState = state;
		this.connectionStateEmitter.fire(state);
	}

	private observeConnection(api: IRemoteAgentApi): void {
		const subscription = api.onDidChangeConnection(connection => {
			if (this.disposed) return;
			this.connectionRevision += 1;
			this.setConnection(connection);
		});
		this.defer(() => subscription.dispose());
		const readRevision = this.connectionRevision;
		void Promise.resolve().then(() => this.disposed ? undefined : api.getConnection()).then(connection => {
			if (!this.disposed && connection !== undefined && this.connectionRevision === readRevision) this.setConnection(connection);
		}, error => {
			if (!this.disposed && this.connectionRevision === readRevision) defaultConnectionReadErrorHandler(error);
		});
	}

	private setConnection(connection: RemoteAgentConnection): void {
		if (sameConnection(this._connection, connection)) return;
		this._connection = Object.freeze({ ...connection });
		this.connectionEmitter.fire(this._connection);
	}
}

function toRemoteConnectionState(state: AppServerConnectionState): RemoteConnectionState {
	switch (state) {
		case "starting":
		case "initializing":
			return "connecting";
		case "ready":
			return "connected";
		case "stopping":
			return "disconnecting";
		case "restarting":
			return "reconnecting";
		case "stopped":
		case "crashed":
			return "disconnected";
	}
}

function defaultReadErrorHandler(error: unknown): void {
	console.error("Failed to read remote agent connection state", error);
}

function defaultConnectionReadErrorHandler(error: unknown): void {
	console.error("Failed to read remote agent connection metadata", error);
}

function sameConnection(first: RemoteAgentConnection | undefined, second: RemoteAgentConnection): boolean {
	if (!first || first.kind !== second.kind || first.generation !== second.generation) return false;
	return first.kind === "local" || second.kind === "local" || first.authority === second.authority;
}
