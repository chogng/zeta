import type { ZetaRendererApi } from "../../app-server/common/renderer-api.js";
import { type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import type { ITerminalProcessCloseOptions, ITerminalProcessCreateOptions, ITerminalProcessCreation, ITerminalProcessProfile, ITerminalProcessReadOptions, ITerminalProcessReadResult, ITerminalProcessResizeOptions, ITerminalProcessService, ITerminalProcessWriteOptions, TerminalProcessConnectionState } from "../common/terminalProcess.js";

/**
 * Browser terminal-process service backed by the renderer's trusted capability.
 *
 * Workbench terminal consumers depend on ITerminalService and never access
 * this transport-facing process service or renderer capability directly.
 */
export class BrowserTerminalProcessService implements ITerminalProcessService {
  constructor(private readonly api: Pick<ZetaRendererApi, "appServer" | "terminal">) {}

  async listProfiles(): Promise<readonly ITerminalProcessProfile[]> {
    const result = await this.api.terminal.listProfiles();
    return result.profiles;
  }

  create(options: ITerminalProcessCreateOptions): Promise<ITerminalProcessCreation> {
    return this.api.terminal.create(options);
  }

  write(options: ITerminalProcessWriteOptions): Promise<void> {
    return this.api.terminal.write(options);
  }

  resize(options: ITerminalProcessResizeOptions): Promise<void> {
    return this.api.terminal.resize(options);
  }

  read(options: ITerminalProcessReadOptions): Promise<ITerminalProcessReadResult> {
    return this.api.terminal.read(options);
  }

  close(options: ITerminalProcessCloseOptions): Promise<void> {
    return this.api.terminal.close(options);
  }

  getConnectionState(): Promise<TerminalProcessConnectionState> {
    return this.api.appServer.getConnectionState();
  }

  onConnectionState(listener: (state: TerminalProcessConnectionState) => void): IDisposable {
    const subscription = this.api.appServer.onConnectionState(listener);
    return toDisposable(() => subscription.dispose());
  }
}
