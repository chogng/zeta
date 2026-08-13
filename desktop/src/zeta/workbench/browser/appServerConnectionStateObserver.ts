import { DisposableOwner } from "../../base/common/lifecycle.js";
import type { AppServerConnectionState, IAppServerApi } from "../../platform/app-server/common/appServerApi.js";

export interface AppServerConnectionStateObserverOptions {
  readonly api: IAppServerApi;
  readonly onState: (state: AppServerConnectionState) => void;
  readonly onReadError: (error: unknown) => void;
}

/** Delivers one ordered App Server state stream without letting the initial read overwrite newer events. */
export class AppServerConnectionStateObserver extends DisposableOwner {
  private revision = 0;
  private disposed = false;

  constructor(options: AppServerConnectionStateObserverOptions) {
    super();
    const subscription = options.api.onConnectionState(state => {
      if (this.disposed) return;
      this.revision += 1;
      options.onState(state);
    });
    this.defer(() => subscription.dispose());
    const readRevision = this.revision;
    void Promise.resolve()
      .then(() => this.disposed ? undefined : options.api.getConnectionState())
      .then(state => {
        if (!this.disposed && state !== undefined && this.revision === readRevision) options.onState(state);
      }, error => {
        if (!this.disposed && this.revision === readRevision) options.onReadError(error);
      });
    this.defer(() => { this.disposed = true; });
  }
}
