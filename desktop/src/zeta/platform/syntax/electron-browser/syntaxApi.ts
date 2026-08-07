import type { SyntaxAnalyzeResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ISyntaxApi } from "../common/syntaxApi.js";

export function createSyntaxApi(): ISyntaxApi {
  return {
    analyze: params => invoke<SyntaxAnalyzeResult>("zeta:syntax:analyze", params),
  };
}
