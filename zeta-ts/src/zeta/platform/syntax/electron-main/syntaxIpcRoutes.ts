import { APP_SERVER_METHODS, type SyntaxAnalyzeParams, type SyntaxRangeDto, type SyntaxSelectionRangesParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const MAX_SYNTAX_INPUT_BYTES = 4 * 1024 * 1024;
const SUPPORTED_LANGUAGES = new Set(["javascript", "javascriptreact", "json", "jsonc", "rust", "shell", "typescript", "typescriptreact"]);

/** Exact-shape IPC route for the Rust-backed source syntax projection. */
export function syntaxIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({
			channel: "zeta:syntax:analyze",
			validate: syntaxAnalyzeParams,
			invoke: params => supervisor.request(APP_SERVER_METHODS["syntax/analyze"], params),
		}),
		route({
			channel: "zeta:syntax:selectionRanges",
			validate: syntaxSelectionRangesParams,
			invoke: params => supervisor.request(APP_SERVER_METHODS["syntax/selectionRanges"], params),
		}),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return {
		channel: definition.channel,
		validate: definition.validate,
		invoke: params => definition.invoke(params as P),
	};
}

function syntaxAnalyzeParams(value: unknown): SyntaxAnalyzeParams {
	return syntaxDocumentParams(record(value, ["language", "revision", "text"]));
}

function syntaxSelectionRangesParams(value: unknown): SyntaxSelectionRangesParams {
	const params = record(value, ["language", "revision", "text", "ranges"]);
	if (!Array.isArray(params.ranges) || params.ranges.length > 1_024) throw new Error("ranges must be an array of at most 1024 syntax ranges");
	return {
		...syntaxDocumentParams(params),
		ranges: params.ranges.map((range, index) => syntaxRange(range, `ranges[${index}]`)),
	};
}

function syntaxDocumentParams(params: Record<string, unknown>): SyntaxAnalyzeParams {
	const language = string(params.language, "language");
	if (!SUPPORTED_LANGUAGES.has(language)) {
		throw new Error("language must be one of javascript, javascriptreact, json, jsonc, rust, shell, typescript, or typescriptreact");
	}
	const text = string(params.text, "text");
	if (new TextEncoder().encode(text).byteLength > MAX_SYNTAX_INPUT_BYTES) {
		throw new Error(`text must not exceed ${MAX_SYNTAX_INPUT_BYTES} UTF-8 bytes`);
	}
	return {
		language: language as SyntaxAnalyzeParams["language"],
		revision: nonNegativeInteger(params.revision, "revision"),
		text,
	};
}

function syntaxRange(value: unknown, name: string): SyntaxRangeDto {
	const range = record(value, ["start", "end"]);
	return { start: syntaxPosition(range.start, `${name}.start`), end: syntaxPosition(range.end, `${name}.end`) };
}

function syntaxPosition(value: unknown, name: string): SyntaxRangeDto["start"] {
	const position = record(value, ["lineIndex", "columnIndex"]);
	return { lineIndex: nonNegativeInteger(position.lineIndex, `${name}.lineIndex`), columnIndex: nonNegativeInteger(position.columnIndex, `${name}.columnIndex`) };
}
