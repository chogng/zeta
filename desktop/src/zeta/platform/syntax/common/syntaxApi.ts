import type { SyntaxAnalyzeParams, SyntaxAnalyzeResult } from "../../../../../generated/app-server/types.js";

/** Transport-neutral entry point for bounded, authoritative source syntax analysis. */
export interface ISyntaxApi {
  analyze(params: SyntaxAnalyzeParams): Promise<SyntaxAnalyzeResult>;
}
