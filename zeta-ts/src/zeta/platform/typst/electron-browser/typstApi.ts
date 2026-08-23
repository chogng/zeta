import type { TypstCompileResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ITypstApi } from "../common/typstApi.js";

export function createTypstApi(): ITypstApi {
	return { compile: (params) => invoke<TypstCompileResult>("zeta:typst:compile", params) };
}
