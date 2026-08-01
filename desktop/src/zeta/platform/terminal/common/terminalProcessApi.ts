import type { TerminalCloseParams, TerminalCreateParams, TerminalCreateResult, TerminalProfileListResult, TerminalReadParams, TerminalReadResult, TerminalResizeParams, TerminalWriteParams } from "../../../../../generated/app-server/types.js";

export interface ITerminalProcessApi {
  listProfiles(): Promise<TerminalProfileListResult>;
  create(params: TerminalCreateParams): Promise<TerminalCreateResult>;
  write(params: TerminalWriteParams): Promise<void>;
  resize(params: TerminalResizeParams): Promise<void>;
  read(params: TerminalReadParams): Promise<TerminalReadResult>;
  close(params: TerminalCloseParams): Promise<void>;
}
