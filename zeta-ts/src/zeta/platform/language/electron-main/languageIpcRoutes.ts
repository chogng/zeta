import { APP_SERVER_METHODS, type LanguageCloseParams, type LanguageCodeActionDto, type LanguageCodeActionsParams, type LanguageCodeLensDto, type LanguageColorDto, type LanguageColorPresentationsParams, type LanguageCommandDto, type LanguageCompletionsParams, type LanguageDocumentDiagnosticsParams, type LanguageDocumentFeaturesParams, type LanguageDocumentFormattingParams, type LanguageDocumentLinkDto, type LanguageExecuteCommandParams, type LanguageHierarchyItemDto, type LanguageHierarchyParams, type LanguageHoverParams, type LanguageInlayHintsParams, type LanguageLinkedEditingRangesParams, type LanguageLocationsParams, type LanguagePrepareRenameParams, type LanguageRangeFormattingParams, type LanguageRenameParams, type LanguageResolveCodeActionParams, type LanguageResolveCodeLensParams, type LanguageResolveCompletionParams, type LanguageResolveDocumentLinkParams, type LanguageSemanticTokensParams, type LanguageSignatureHelpParams, type LanguageSynchronizeParams, type LanguageWorkspaceDiagnosticsParams, type LanguageWorkspaceSymbolsParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { boolean, nonNegativeInteger, record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

const KINDS = new Set(["declaration", "definition", "implementation", "typeDefinition", "references"]);
const HIERARCHY_KINDS = new Set(["prepareCall", "incomingCalls", "outgoingCalls", "prepareType", "supertypes", "subtypes"]);
const MAX_LANGUAGE_INPUT_BYTES = 10 * 1024 * 1024;

export function languageIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:language:synchronize", validate: languageSynchronizeParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/synchronize"], params) }),
		route({ channel: "zeta:language:close", validate: languageCloseParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/close"], params) }),
		route({ channel: "zeta:language:hover", validate: languageHoverParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/hover"], params) }),
		route({ channel: "zeta:language:completions", validate: languageCompletionsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/completions"], params) }),
		route({ channel: "zeta:language:resolveCompletion", validate: languageResolveCompletionParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/resolveCompletion"], params) }),
		route({ channel: "zeta:language:executeCommand", validate: languageExecuteCommandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/executeCommand"], params) }),
		route({ channel: "zeta:language:documentDiagnostics", validate: languageDocumentDiagnosticsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/documentDiagnostics"], params) }),
		route({ channel: "zeta:language:workspaceDiagnostics", validate: languageWorkspaceDiagnosticsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/workspaceDiagnostics"], params) }),
		route({ channel: "zeta:language:formatDocument", validate: languageDocumentFormattingParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/formatDocument"], params) }),
		route({ channel: "zeta:language:formatRange", validate: languageRangeFormattingParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/formatRange"], params) }),
		route({ channel: "zeta:language:signatureHelp", validate: languageSignatureHelpParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/signatureHelp"], params) }),
		route({ channel: "zeta:language:inlayHints", validate: languageInlayHintsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/inlayHints"], params) }),
		route({ channel: "zeta:language:linkedEditingRanges", validate: languageLinkedEditingRangesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/linkedEditingRanges"], params) }),
		route({ channel: "zeta:language:semanticTokens", validate: languageSemanticTokensParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/semanticTokens"], params) }),
		route({ channel: "zeta:language:documentSymbols", validate: languageDocumentFeaturesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/documentSymbols"], params) }),
		route({ channel: "zeta:language:codeLenses", validate: languageDocumentFeaturesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/codeLenses"], params) }),
		route({ channel: "zeta:language:resolveCodeLens", validate: languageResolveCodeLensParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/resolveCodeLens"], params) }),
		route({ channel: "zeta:language:documentLinks", validate: languageDocumentFeaturesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/documentLinks"], params) }),
		route({ channel: "zeta:language:resolveDocumentLink", validate: languageResolveDocumentLinkParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/resolveDocumentLink"], params) }),
		route({ channel: "zeta:language:documentColors", validate: languageDocumentFeaturesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/documentColors"], params) }),
		route({ channel: "zeta:language:colorPresentations", validate: languageColorPresentationsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/colorPresentations"], params) }),
		route({ channel: "zeta:language:foldingRanges", validate: languageDocumentFeaturesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/foldingRanges"], params) }),
		route({ channel: "zeta:language:locations", validate: languageLocationsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/locations"], params) }),
		route({ channel: "zeta:language:hierarchy", validate: languageHierarchyParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/hierarchy"], params) }),
		route({ channel: "zeta:language:workspaceSymbols", validate: languageWorkspaceSymbolsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/workspaceSymbols"], params) }),
		route({ channel: "zeta:language:prepareRename", validate: languagePrepareRenameParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/prepareRename"], params) }),
		route({ channel: "zeta:language:rename", validate: languageRenameParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/rename"], params) }),
		route({ channel: "zeta:language:codeActions", validate: languageCodeActionsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/codeActions"], params) }),
		route({ channel: "zeta:language:resolveCodeAction", validate: languageResolveCodeActionParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/resolveCodeAction"], params) }),
	];
}

function languageSynchronizeParams(value: unknown): LanguageSynchronizeParams {
	const params = record(value, ["document"]);
	return { document: languageDocument(params.document) };
}

function languageCloseParams(value: unknown): LanguageCloseParams {
	const params = record(value, ["path"]);
	return { path: string(params.path, "path") };
}

function languageHoverParams(value: unknown): LanguageHoverParams {
	const params = record(value, ["document", "position"]);
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position") };
}

function languageCompletionsParams(value: unknown): LanguageCompletionsParams {
	const params = record(value, ["document", "position", "triggerKind", "triggerCharacter"]);
	const triggerKind = string(params.triggerKind, "triggerKind");
	if (!["invoke", "triggerCharacter", "incompleteRefresh"].includes(triggerKind)) throw new Error("triggerKind must be a supported completion trigger");
	if (params.triggerCharacter !== null && typeof params.triggerCharacter !== "string") throw new Error("triggerCharacter must be a string or null");
	if ((triggerKind === "triggerCharacter") !== (params.triggerCharacter !== null)) throw new Error("triggerCharacter is required only for trigger-character completion");
	if (typeof params.triggerCharacter === "string" && (params.triggerCharacter === "\n" || params.triggerCharacter === "\r" || [...params.triggerCharacter].length !== 1)) throw new Error("triggerCharacter must contain one non-line-break character");
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position"), triggerKind: triggerKind as LanguageCompletionsParams["triggerKind"], triggerCharacter: params.triggerCharacter as string | null };
}

function languagePrepareRenameParams(value: unknown): LanguagePrepareRenameParams {
	const params = record(value, ["document", "position"]);
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position") };
}

function languageDocumentFormattingParams(value: unknown): LanguageDocumentFormattingParams {
	const params = record(value, ["document", "options"]);
	return { document: languageDocument(params.document), options: languageFormattingOptions(params.options) };
}

function languageRangeFormattingParams(value: unknown): LanguageRangeFormattingParams {
	const params = record(value, ["document", "range", "options"]);
	return { document: languageDocument(params.document), range: languageRange(params.range, "range"), options: languageFormattingOptions(params.options) };
}

function languageFormattingOptions(value: unknown): LanguageDocumentFormattingParams["options"] {
	const options = record(value, ["tabSize", "insertSpaces", "trimTrailingWhitespace"]);
	const tabSize = nonNegativeInteger(options.tabSize, "options.tabSize");
	if (tabSize === 0 || tabSize > 256) throw new Error("options.tabSize must be between 1 and 256");
	if (options.trimTrailingWhitespace !== null && typeof options.trimTrailingWhitespace !== "boolean") throw new Error("options.trimTrailingWhitespace must be a boolean or null");
	return { tabSize, insertSpaces: boolean(options.insertSpaces, "options.insertSpaces"), trimTrailingWhitespace: options.trimTrailingWhitespace as boolean | null };
}

function languageSignatureHelpParams(value: unknown): LanguageSignatureHelpParams {
	const params = record(value, ["document", "position", "triggerKind", "triggerCharacter"]);
	const triggerKind = string(params.triggerKind, "triggerKind");
	if (!["invoke", "triggerCharacter", "contentChange"].includes(triggerKind)) throw new Error("triggerKind must be a supported signature-help trigger");
	if (params.triggerCharacter !== null && typeof params.triggerCharacter !== "string") throw new Error("triggerCharacter must be a string or null");
	if ((triggerKind === "triggerCharacter") !== (params.triggerCharacter !== null)) throw new Error("triggerCharacter is required only for trigger-character signature help");
	if (typeof params.triggerCharacter === "string" && (params.triggerCharacter === "\n" || params.triggerCharacter === "\r" || [...params.triggerCharacter].length !== 1)) throw new Error("triggerCharacter must contain one non-line-break character");
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position"), triggerKind: triggerKind as LanguageSignatureHelpParams["triggerKind"], triggerCharacter: params.triggerCharacter as string | null };
}

function languageInlayHintsParams(value: unknown): LanguageInlayHintsParams {
	const params = record(value, ["document", "range"]);
	return { document: languageDocument(params.document), range: languageRange(params.range, "range") };
}

function languageLinkedEditingRangesParams(value: unknown): LanguageLinkedEditingRangesParams {
	const params = record(value, ["document", "position"]);
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position") };
}

function languageResolveCompletionParams(value: unknown): LanguageResolveCompletionParams {
	const params = record(value, ["document", "providerData"]);
	return { document: languageDocument(params.document), providerData: params.providerData };
}

function languageExecuteCommandParams(value: unknown): LanguageExecuteCommandParams {
	const params = record(value, ["document", "command"]);
	return { document: languageDocument(params.document), command: languageCommand(params.command, "command") };
}

function languageDocumentDiagnosticsParams(value: unknown): LanguageDocumentDiagnosticsParams {
	const params = record(value, ["document"]);
	return { document: languageDocument(params.document) };
}

function languageWorkspaceDiagnosticsParams(value: unknown): LanguageWorkspaceDiagnosticsParams {
	const params = record(value, ["languageId"]);
	return { languageId: string(params.languageId, "languageId") };
}

function languageSemanticTokensParams(value: unknown): LanguageSemanticTokensParams {
	const params = record(value, ["document"]);
	return { document: languageDocument(params.document) };
}

function languageDocumentFeaturesParams(value: unknown): LanguageDocumentFeaturesParams {
	const params = record(value, ["document"]);
	return { document: languageDocument(params.document) };
}

function languageResolveCodeLensParams(value: unknown): LanguageResolveCodeLensParams {
	const params = record(value, ["document", "lens"]);
	return { document: languageDocument(params.document), lens: languageCodeLens(params.lens) };
}

function languageResolveDocumentLinkParams(value: unknown): LanguageResolveDocumentLinkParams {
	const params = record(value, ["document", "link"]);
	return { document: languageDocument(params.document), link: languageDocumentLink(params.link) };
}

function languageColorPresentationsParams(value: unknown): LanguageColorPresentationsParams {
	const params = record(value, ["document", "range", "color"]);
	return { document: languageDocument(params.document), range: languageRange(params.range, "range"), color: languageColor(params.color) };
}

function languageCodeLens(value: unknown): LanguageCodeLensDto {
	const lens = record(value, ["range", "command", "providerData"]);
	const command = lens.command === null ? null : languageCommand(lens.command, "lens.command");
	return { range: languageRange(lens.range, "lens.range"), command, providerData: lens.providerData };
}

function languageCommand(value: unknown, field: string): LanguageCommandDto {
	const command = record(value, ["id", "title", "arguments"]);
	if (!Array.isArray(command.arguments)) throw new Error(`${field}.arguments must be an array`);
	return { id: string(command.id, `${field}.id`), title: string(command.title, `${field}.title`), arguments: command.arguments };
}

function languageDocumentLink(value: unknown): LanguageDocumentLinkDto {
	const link = record(value, ["range", "target", "tooltip", "providerData"]);
	if (link.target !== null && typeof link.target !== "string") throw new Error("link.target must be a string or null");
	if (link.tooltip !== null && typeof link.tooltip !== "string") throw new Error("link.tooltip must be a string or null");
	return { range: languageRange(link.range, "link.range"), target: link.target as string | null, tooltip: link.tooltip as string | null, providerData: link.providerData };
}

function languageColor(value: unknown): LanguageColorDto {
	const color = record(value, ["red", "green", "blue", "alpha"]);
	const component = (candidate: unknown, field: string): number => {
		const parsed = nonNegativeInteger(candidate, field);
		if (parsed > 255) throw new Error(`${field} must not exceed 255`);
		return parsed;
	};
	return { red: component(color.red, "color.red"), green: component(color.green, "color.green"), blue: component(color.blue, "color.blue"), alpha: component(color.alpha, "color.alpha") };
}

function languageRenameParams(value: unknown): LanguageRenameParams {
	const params = record(value, ["document", "position", "newName"]);
	const newName = string(params.newName, "newName");
	if (newName.length === 0 || newName.length > 1024) throw new Error("newName must contain 1-1024 characters");
	return { document: languageDocument(params.document), position: languagePosition(params.position, "position"), newName };
}

function languageCodeActionsParams(value: unknown): LanguageCodeActionsParams {
	const params = record(value, ["document", "range", "diagnostics", "only"]);
	if (!Array.isArray(params.diagnostics) || !Array.isArray(params.only)) throw new Error("diagnostics and only must be arrays");
	return {
		document: languageDocument(params.document),
		range: languageRange(params.range, "range"),
		diagnostics: params.diagnostics.map((value, index) => {
			const diagnostic = record(value, ["range", "severity", "message", "code", "source"]);
			const severity = string(diagnostic.severity, `diagnostics[${index}].severity`);
			if (!["error", "warning", "information", "hint"].includes(severity)) throw new Error("diagnostic severity is invalid");
			if (diagnostic.source !== null && typeof diagnostic.source !== "string") throw new Error("diagnostic source must be a string or null");
			return { range: languageRange(diagnostic.range, `diagnostics[${index}].range`), severity: severity as LanguageCodeActionsParams["diagnostics"][number]["severity"], message: string(diagnostic.message, `diagnostics[${index}].message`), code: diagnostic.code, source: diagnostic.source as string | null };
		}),
		only: params.only.map((value, index) => string(value, `only[${index}]`)),
	};
}

function languageResolveCodeActionParams(value: unknown): LanguageResolveCodeActionParams {
	const params = record(value, ["document", "providerData"]);
	return { document: languageDocument(params.document), providerData: params.providerData };
}

function languageWorkspaceSymbolsParams(value: unknown): LanguageWorkspaceSymbolsParams {
	const params = record(value, ["languageId", "query"]);
	const query = string(params.query, "query");
	if (query.length > 1024) throw new Error("query must not exceed 1024 characters");
	return { languageId: string(params.languageId, "languageId"), query };
}

function languageHierarchyParams(value: unknown): LanguageHierarchyParams {
	const params = record(value, ["document", "kind", "position", "item"]);
	const kind = string(params.kind, "kind");
	if (!HIERARCHY_KINDS.has(kind)) throw new Error("kind must be a supported language hierarchy operation");
	const isPrepare = kind === "prepareCall" || kind === "prepareType";
	if (isPrepare === (params.position === null)) throw new Error("prepare hierarchy requests require position and follow-up requests require item");
	if (isPrepare === (params.item !== null)) throw new Error("prepare hierarchy requests must not include item and follow-up requests must include item");
	return {
		document: languageDocument(params.document),
		kind: kind as LanguageHierarchyParams["kind"],
		position: params.position === null ? null : languagePosition(params.position, "position"),
		item: params.item === null ? null : languageHierarchyItem(params.item),
	};
}

function languageHierarchyItem(value: unknown): LanguageHierarchyItemDto {
	const item = record(value, ["name", "symbolKind", "detail", "path", "range", "selectionRange", "data"]);
	if (item.detail !== null && typeof item.detail !== "string") throw new Error("item.detail must be a string or null");
	return {
		name: string(item.name, "item.name"),
		symbolKind: nonNegativeInteger(item.symbolKind, "item.symbolKind"),
		detail: item.detail as string | null,
		path: string(item.path, "item.path"),
		range: languageRange(item.range, "item.range"),
		selectionRange: languageRange(item.selectionRange, "item.selectionRange"),
		data: item.data,
	};
}

function languageDocument(value: unknown): LanguageLocationsParams["document"] {
	const document = record(value, ["path", "languageId", "revision", "text"]);
	const text = string(document.text, "document.text");
	if (VSBuffer.fromString(text).byteLength > MAX_LANGUAGE_INPUT_BYTES) throw new Error(`document.text must not exceed ${MAX_LANGUAGE_INPUT_BYTES} UTF-8 bytes`);
	return { path: string(document.path, "document.path"), languageId: string(document.languageId, "document.languageId"), revision: nonNegativeInteger(document.revision, "document.revision"), text };
}

function languagePosition(value: unknown, field: string): LanguageLocationsParams["position"] {
	const position = record(value, ["lineIndex", "columnIndex"]);
	return { lineIndex: nonNegativeInteger(position.lineIndex, `${field}.lineIndex`), columnIndex: nonNegativeInteger(position.columnIndex, `${field}.columnIndex`) };
}

function languageRange(value: unknown, field: string): LanguageHierarchyItemDto["range"] {
	const range = record(value, ["start", "end"]);
	return { start: languagePosition(range.start, `${field}.start`), end: languagePosition(range.end, `${field}.end`) };
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function languageLocationsParams(value: unknown): LanguageLocationsParams {
	const params = record(value, ["document", "position", "kind", "includeDeclaration"]);
	const kind = string(params.kind, "kind");
	if (!KINDS.has(kind)) throw new Error("kind must be a supported language location operation");
	return {
		document: languageDocument(params.document),
		position: languagePosition(params.position, "position"),
		kind: kind as LanguageLocationsParams["kind"],
		includeDeclaration: boolean(params.includeDeclaration, "includeDeclaration"),
	};
}
