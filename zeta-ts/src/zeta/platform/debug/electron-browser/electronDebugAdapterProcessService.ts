import { type DebugAdapterCloseParams, type DebugAdapterReadResult, type DebugAdapterSendParams, type DebugAdapterStartParams, type DebugAdapterStartResult } from "../../../../../generated/app-server/types.js";
import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import { type IAppServerApi } from "../../app-server/common/appServerApi.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import { type IDebugAdapterProcessReadResult, type IDebugAdapterProcessService, type IDebugAdapterProcessStartOptions } from "../common/debugAdapterProcessService.js";
import { type RendererHostCapabilities } from "../../renderer/common/rendererHost.js";

/** Electron renderer adapter for App Server-owned DAP processes. */
export class ElectronDebugAdapterProcessService implements IDebugAdapterProcessService {
  constructor(private readonly appServer: IAppServerApi) {}

  async start(options: IDebugAdapterProcessStartOptions): Promise<string> {
    const params: DebugAdapterStartParams = { program: options.program, arguments: [...options.arguments] };
    return (await invoke<DebugAdapterStartResult>("zeta:debug-adapter:start", params)).sessionId;
  }

  send(sessionId: string, message: unknown): Promise<void> {
    const params: DebugAdapterSendParams = { sessionId, message };
    return invoke<void>("zeta:debug-adapter:send", params);
  }

  read(sessionId: string, afterSequence: number, maxMessages: number): Promise<IDebugAdapterProcessReadResult> {
    return invoke<DebugAdapterReadResult>("zeta:debug-adapter:read", { sessionId, afterSequence, maxMessages });
  }

  close(sessionId: string): Promise<void> {
    const params: DebugAdapterCloseParams = { sessionId };
    return invoke<void>("zeta:debug-adapter:close", params);
  }

  getConnectionState() { return this.appServer.getConnectionState(); }

  onConnectionState(listener: Parameters<IAppServerApi["onConnectionState"]>[0]): IDisposable {
    const subscription = this.appServer.onConnectionState(listener);
    return toDisposable(() => subscription.dispose());
  }
}

/** Code product contribution for the Electron renderer host. */
export function createElectronDebugAdapterCapability(appServer: IAppServerApi): RendererHostCapabilities {
  return { debugAdapter: new ElectronDebugAdapterProcessService(appServer) };
}
