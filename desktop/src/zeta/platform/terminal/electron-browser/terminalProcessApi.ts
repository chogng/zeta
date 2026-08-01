import type { TerminalCreateResult, TerminalProfileListResult, TerminalReadResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ITerminalProcessApi } from "../common/terminalProcessApi.js";

export function createTerminalProcessApi(): ITerminalProcessApi {
  return {
    listProfiles: () => invoke<TerminalProfileListResult>("zeta:terminal:profile-list"),
    create: (params) => invoke<TerminalCreateResult>("zeta:terminal:create", params),
    write: (params) => invoke<void>("zeta:terminal:write", params),
    resize: (params) => invoke<void>("zeta:terminal:resize", params),
    read: (params) => invoke<TerminalReadResult>("zeta:terminal:read", params),
    close: (params) => invoke<void>("zeta:terminal:close", params),
  };
}
