import type { TerminalCloseParams, TerminalCreateParams, TerminalCreateResult, TerminalProfileListResult, TerminalReadParams, TerminalReadResult, TerminalResizeParams, TerminalWriteParams } from "../../../../../../generated/app-server/types.js";
import type { AppServerConnectionState, ZetaRendererApi } from "../../../../platform/app-server/common/renderer-api.js";
import { type IDisposable, toDisposable } from "../../../../base/common/lifecycle.js";

/** Narrow transport contract consumed by the Workbench terminal service. */
export interface ITerminalBackend {
  listProfiles(): Promise<TerminalProfileListResult>;
  create(params: TerminalCreateParams): Promise<TerminalCreateResult>;
  write(params: TerminalWriteParams): Promise<void>;
  resize(params: TerminalResizeParams): Promise<void>;
  read(params: TerminalReadParams): Promise<TerminalReadResult>;
  close(params: TerminalCloseParams): Promise<void>;
  getConnectionState(): Promise<AppServerConnectionState>;
  onConnectionState(listener: (state: AppServerConnectionState) => void): IDisposable;
}

/**
 * Typed App Server adapter kept separate from Workbench terminal semantics.
 *
 * The renderer API already crosses the trusted preload boundary, so this
 * adapter only establishes the narrow backend shape needed by TerminalService.
 */
export class AppServerTerminalBackend implements ITerminalBackend {
  constructor(private readonly api: Pick<ZetaRendererApi, "appServer" | "terminal">) {}

  listProfiles(): Promise<TerminalProfileListResult> {
    return this.api.terminal.listProfiles();
  }

  create(params: TerminalCreateParams): Promise<TerminalCreateResult> {
    return this.api.terminal.create(params);
  }

  write(params: TerminalWriteParams): Promise<void> {
    return this.api.terminal.write(params);
  }

  resize(params: TerminalResizeParams): Promise<void> {
    return this.api.terminal.resize(params);
  }

  read(params: TerminalReadParams): Promise<TerminalReadResult> {
    return this.api.terminal.read(params);
  }

  close(params: TerminalCloseParams): Promise<void> {
    return this.api.terminal.close(params);
  }

  getConnectionState(): Promise<AppServerConnectionState> {
    return this.api.appServer.getConnectionState();
  }

  onConnectionState(listener: (state: AppServerConnectionState) => void): IDisposable {
    const subscription = this.api.appServer.onConnectionState(listener);
    return toDisposable(() => subscription.dispose());
  }
}
