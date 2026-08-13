import { APP_SERVER_METHODS, type LanguageCloseParams, type LanguageCodeActionDto, type LanguageCodeActionsParams, type LanguageCompletionsParams, type LanguageDocumentFormattingParams, type LanguageHierarchyItemDto, type LanguageHierarchyParams, type LanguageHoverParams, type LanguageInlayHintsParams, type LanguageLinkedEditingRangesParams, type LanguageLocationsParams, type LanguagePrepareRenameParams, type LanguageRangeFormattingParams, type LanguageRenameParams, type LanguageResolveCodeActionParams, type LanguageSignatureHelpParams, type LanguageSynchronizeParams, type LanguageWorkspaceSymbolsParams } from "../../../../../generated/app-server/types.js";
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
    route({ channel: "zeta:language:formatDocument", validate: languageDocumentFormattingParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/formatDocument"], params) }),
    route({ channel: "zeta:language:formatRange", validate: languageRangeFormattingParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/formatRange"], params) }),
    route({ channel: "zeta:language:signatureHelp", validate: languageSignatureHelpParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/signatureHelp"], params) }),
    route({ channel: "zeta:language:inlayHints", validate: languageInlayHintsParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/inlayHints"], params) }),
    route({ channel: "zeta:language:linkedEditingRanges", validate: languageLinkedEditingRangesParams, invoke: params => supervisor.request(APP_SERVER_METHODS["language/linkedEditingRanges"], params) }),
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
  if (new TextEncoder().encode(text).byteLength > MAX_LANGUAGE_INPUT_BYTES) throw new Error(`document.text must not exceed ${MAX_LANGUAGE_INPUT_BYTES} UTF-8 bytes`);
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
