import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { AppServerConnectionState, IAppServerApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { RemoteConnectionState } from "../../../../platform/remote/common/remote.js";
import type { IRemoteAgentService } from "../common/remoteAgentService.js";

export interface AppServerRemoteAgentServiceOptions {
  readonly api: IAppServerApi;
  readonly onReadError?: (error: unknown) => void;
}

/** Adapts the App Server supervisor state into the Workbench remote-agent contract. */
export class AppServerRemoteAgentService extends DisposableOwner implements IRemoteAgentService {
  private readonly connectionStateEmitter = this.own(new Emitter<RemoteConnectionState>());
  private revision = 0;
  private disposed = false;
  private _connectionState: RemoteConnectionState | undefined;

  readonly onDidChangeConnectionState = this.connectionStateEmitter.event;

  constructor(options: AppServerRemoteAgentServiceOptions) {
    super();
    const subscription = options.api.onConnectionState(state => {
      if (this.disposed) return;
      this.revision += 1;
      this.acceptState(state);
    });
    this.defer(() => subscription.dispose());
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

  private acceptState(state: AppServerConnectionState): void {
    this.setConnectionState(toRemoteConnectionState(state));
  }

  private setConnectionState(state: RemoteConnectionState): void {
    if (this._connectionState === state) return;
    this._connectionState = state;
    this.connectionStateEmitter.fire(state);
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
