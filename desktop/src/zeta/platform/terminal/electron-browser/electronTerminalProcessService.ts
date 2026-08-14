import type { TerminalCloseParams, TerminalCreateParams, TerminalProfileListResult, TerminalReadParams, TerminalReadResult, TerminalResizeParams, TerminalWriteParams } from "../../../../../generated/app-server/types.js";
import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import type { IAppServerApi } from "../../app-server/common/appServerApi.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ITerminalProcessCloseOptions, ITerminalProcessCreateOptions, ITerminalProcessCreation, ITerminalProcessProfile, ITerminalProcessReadOptions, ITerminalProcessReadResult, ITerminalProcessResizeOptions, ITerminalProcessService, ITerminalProcessWriteOptions, TerminalProcessConnectionState } from "../common/terminalProcessService.js";

/** Electron renderer implementation of the terminal process service. */
export class ElectronTerminalProcessService implements ITerminalProcessService {
  constructor(private readonly appServerApi: IAppServerApi) {}

  async listProfiles(): Promise<readonly ITerminalProcessProfile[]> {
    const result = await invoke<TerminalProfileListResult>("zeta:terminal:profile-list");
    return result.profiles;
  }

  create(options: ITerminalProcessCreateOptions): Promise<ITerminalProcessCreation> {
    const params: TerminalCreateParams = { ...options, lifecycle: { type: "connectionOwned" } };
    return invoke<ITerminalProcessCreation>("zeta:terminal:create", params);
  }

  write(options: ITerminalProcessWriteOptions): Promise<void> {
    const params: TerminalWriteParams = options;
    return invoke<void>("zeta:terminal:write", params);
  }

  resize(options: ITerminalProcessResizeOptions): Promise<void> {
    const params: TerminalResizeParams = options;
    return invoke<void>("zeta:terminal:resize", params);
  }

  read(options: ITerminalProcessReadOptions): Promise<ITerminalProcessReadResult> {
    const params: TerminalReadParams = options;
    return invoke<TerminalReadResult>("zeta:terminal:read", params);
  }

  close(options: ITerminalProcessCloseOptions): Promise<void> {
    const params: TerminalCloseParams = options;
    return invoke<void>("zeta:terminal:close", params);
  }

  getConnectionState(): Promise<TerminalProcessConnectionState> {
    return this.appServerApi.getConnectionState();
  }

  onConnectionState(listener: (state: TerminalProcessConnectionState) => void): IDisposable {
    const subscription = this.appServerApi.onConnectionState(listener);
    return toDisposable(() => subscription.dispose());
  }
}
