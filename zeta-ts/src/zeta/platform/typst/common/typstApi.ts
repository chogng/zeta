import type { TypstCompileParams, TypstCompileResult } from "../../../../../generated/app-server/types.js";

export interface ITypstApi {
	compile(params: TypstCompileParams): Promise<TypstCompileResult>;
}
