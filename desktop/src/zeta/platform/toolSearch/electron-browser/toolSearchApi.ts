import type { ConfigCommandResult, ConfigReadResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IToolSearchApi } from "../common/toolSearchApi.js";

export function createToolSearchApi(): IToolSearchApi {
  return {
    readConfig: () => invoke<ConfigReadResult>("zeta:tool-search:config-read"),
    configure: params => invoke<ConfigCommandResult>("zeta:tool-search:configure", params),
  };
}
