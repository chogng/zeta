import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import type { IAppServerApi } from "../../app-server/common/appServerApi.js";
import type { ITerminalProcessApi } from "../common/terminalProcessApi.js";
import type { ITerminalProcessCloseOptions, ITerminalProcessCreateOptions, ITerminalProcessCreation, ITerminalProcessProfile, ITerminalProcessReadOptions, ITerminalProcessReadResult, ITerminalProcessResizeOptions, ITerminalProcessService, ITerminalProcessWriteOptions, TerminalProcessConnectionState } from "../common/terminalProcessService.js";

/**
 * Browser terminal-process service backed by the renderer's trusted capability.
 *
 * Workbench terminal consumers depend on ITerminalService and never access
 * this transport-facing process service or renderer capability directly.
 */
export class TerminalProcessService implements ITerminalProcessService {
  constructor(private readonly api: ITerminalProcessApi, private readonly appServerApi: IAppServerApi) {}

  async listProfiles(): Promise<readonly ITerminalProcessProfile[]> {
    const result = await this.api.listProfiles();
    return result.profiles;
  }

  create(options: ITerminalProcessCreateOptions): Promise<ITerminalProcessCreation> {
    return this.api.create(options);
  }

  write(options: ITerminalProcessWriteOptions): Promise<void> {
    return this.api.write(options);
  }

  resize(options: ITerminalProcessResizeOptions): Promise<void> {
    return this.api.resize(options);
  }

  read(options: ITerminalProcessReadOptions): Promise<ITerminalProcessReadResult> {
    return this.api.read(options);
  }

  close(options: ITerminalProcessCloseOptions): Promise<void> {
    return this.api.close(options);
  }

  getConnectionState(): Promise<TerminalProcessConnectionState> {
    return this.appServerApi.getConnectionState();
  }

  onConnectionState(listener: (state: TerminalProcessConnectionState) => void): IDisposable {
    const subscription = this.appServerApi.onConnectionState(listener);
    return toDisposable(() => subscription.dispose());
  }
}
