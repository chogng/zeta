import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ISyntaxApi, SyntaxAnalyzeResult, SyntaxSelectionRangesResult } from "../common/syntaxApi.js";

export function createSyntaxApi(): ISyntaxApi {
	return {
		analyze: params => invoke<SyntaxAnalyzeResult>("zeta:syntax:analyze", params),
		selectionRanges: params => invoke<SyntaxSelectionRangesResult>("zeta:syntax:selectionRanges", params),
	};
}
