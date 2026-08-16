import { DisposableOwner, toDisposable } from "../../../../base/common/lifecycle.js";
import type { IAppServerApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IWorkbenchWindowService } from "../../../browser/window.js";
import type { IOutputChannel, IOutputService } from "../common/outputService.js";

/** Owns built-in Window and App Server diagnostic Output channels. */
export class SystemOutputService extends DisposableOwner {
  private readonly windowChannel: IOutputChannel;
  private readonly appServerChannel: IOutputChannel;
  private disposed = false;

  constructor(output: IOutputService, appServer: IAppServerApi, windowService: IWorkbenchWindowService) {
    super();
    this.windowChannel = this.own(output.createChannel({ id: "window", label: "Window", kind: "log", source: "core" }));
    this.appServerChannel = this.own(output.createChannel({ id: "app-server", label: "App Server", kind: "log", source: "core" }));
    const targetWindow = windowService.root.ownerDocument.defaultView;
    if (targetWindow) {
      const onError = (event: ErrorEvent): void => this.logWindowError(event);
      const onUnhandledRejection = (event: PromiseRejectionEvent): void => this.logUnhandledRejection(event.reason);
      targetWindow.addEventListener("error", onError);
      targetWindow.addEventListener("unhandledrejection", onUnhandledRejection);
      this.own(toDisposable(() => {
        targetWindow.removeEventListener("error", onError);
        targetWindow.removeEventListener("unhandledrejection", onUnhandledRejection);
      }));
    }
    const connection = appServer.onConnectionState(state => this.appServerChannel.appendLine({ severity: state === "crashed" ? "error" : state === "restarting" ? "warning" : "information", category: "connection", text: `App Server connection is ${state}.` }));
    this.own(toDisposable(() => connection.dispose()));
    void appServer.getConnectionState().then(state => {
      if (!this.disposed) this.appServerChannel.appendLine({ severity: state === "crashed" ? "error" : "information", category: "connection", text: `Initial App Server connection state: ${state}.` });
    }).catch(error => {
      if (!this.disposed) this.appServerChannel.appendLine({ severity: "error", category: "connection", text: `Could not read App Server connection state: ${errorMessage(error)}` });
    });
  }

  override dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    super.dispose();
  }

  private logWindowError(event: ErrorEvent): void {
    const location = event.filename ? ` (${event.filename}:${event.lineno}:${event.colno})` : "";
    this.windowChannel.appendLine({ severity: "error", category: "runtime", text: `${bounded(event.message || errorMessage(event.error))}${location}` });
  }

  private logUnhandledRejection(reason: unknown): void {
    this.windowChannel.appendLine({ severity: "error", category: "runtime", text: `Unhandled promise rejection: ${bounded(errorMessage(reason))}` });
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.stack || error.message;
  return String(error);
}

function bounded(value: string): string {
  return value.slice(0, 16 * 1024);
}
