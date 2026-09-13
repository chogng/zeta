import type { TypstCompileParams, TypstCompileResult } from "../../../../../generated/app-server/index.js";

export interface ITypstApi {
	compile(params: TypstCompileParams): Promise<TypstCompileResult>;
}
