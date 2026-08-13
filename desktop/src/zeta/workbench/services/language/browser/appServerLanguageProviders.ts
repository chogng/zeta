import { DisposableOwner, DisposableStore } from "../../../../base/common/lifecycle.js";
import { URI } from "../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../editor/common/core/text.js";
import { type ILanguageFeaturesService } from "../../../../editor/common/services/languageService.js";
import { LanguageCompletionInsertTextFormat, LanguageCompletionItemKind } from "../../../../editor/common/languages/completion/languageCompletions.js";
import { LanguageCompletionTriggerKind, type LanguageCompletionProvider, type LanguageCompletionProviderRequest } from "../../../../editor/common/languages/completion/languageCompletionProviders.js";
import { type LanguageHoverProvider, type LanguageHoverRequest } from "../../../../editor/contrib/hover/common/hover.js";
import { type LanguageDeclarationProvider, type LanguageDefinitionProvider, type LanguageImplementationProvider, type LanguageLocation, type LanguageLocationRequest, type LanguageReferenceProvider, type LanguageReferenceRequest, type LanguageTypeDefinitionProvider } from "../../../../editor/contrib/gotoSymbol/common/languageNavigation.js";
import { type LanguageCallHierarchyEntry, type LanguageCallHierarchyProvider, type LanguageHierarchyFollowupRequest, type LanguageHierarchyItem, type LanguageHierarchyRequest, type LanguageTypeHierarchyProvider } from "../../../../editor/contrib/callHierarchy/common/languageHierarchy.js";
import { type LanguageCompletionItemKindDto, type LanguageHierarchyItemDto } from "../../../../../../generated/app-server/types.js";
import { type LanguageWorkspaceSymbol, type LanguageWorkspaceSymbolProvider } from "../../../../editor/common/languages/workspaceSymbols.js";
import { type LanguageRenameProvider, type LanguageRenameRequest } from "../../../../editor/contrib/rename/common/rename.js";
import { type LanguageCodeAction, type LanguageCodeActionProvider, type LanguageCodeActionRequest } from "../../../../editor/contrib/codeAction/common/codeAction.js";
import { type LanguageFormattingProvider, type LanguageFormattingRequest } from "../../../../editor/contrib/format/common/formatCommands.js";
import { type LanguageParameterHintsProvider, type LanguageParameterHintsRequest } from "../../../../editor/contrib/parameterHints/common/parameterHints.js";
import { type LanguageInlayHintsProvider, type LanguageInlayHintsRequest } from "../../../../editor/contrib/inlayHints/common/inlayHints.js";
import { type LanguageLinkedEditingProvider, type LanguageLinkedEditingRequest } from "../../../../editor/contrib/linkedEditing/common/linkedEditing.js";
import { LanguageDiagnosticSeverity } from "../../../../editor/common/languages/languageResults.js";
import { type LanguageCodeActionDto, type LanguageWorkspaceEditDto } from "../../../../../../generated/app-server/types.js";
import { type ILanguageApi } from "../../../../platform/language/common/languageApi.js";
import { workspaceRelativePath, workspaceResourceFromPath } from "../../../../platform/files/browser/fileService.js";
import { type IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { APP_SERVER_LANGUAGE_IDS } from "./appServerLanguageSupport.js";

type LocationKind = "declaration" | "definition" | "implementation" | "typeDefinition" | "references";
const APP_SERVER_LANGUAGE_DOCUMENT_MAX_BYTES = 10 * 1024 * 1024;

/** Registers App Server-backed cross-resource providers for Code languages. */
export class AppServerLanguageProviders extends DisposableOwner {
  constructor(languageFeatures: ILanguageFeaturesService, api: ILanguageApi, workspace: IWorkspaceContextService) {
    super();
    const adapter = new AppServerLanguageProvider(api, workspace);
    const registrations = this.own(new DisposableStore());
    registrations.add(languageFeatures.registerHoverProvider(adapter));
    registrations.add(languageFeatures.registerCompletionProvider(adapter));
    registrations.add(languageFeatures.registerDeclarationProvider(adapter));
    registrations.add(languageFeatures.registerDefinitionProvider(adapter));
    registrations.add(languageFeatures.registerImplementationProvider(adapter));
    registrations.add(languageFeatures.registerTypeDefinitionProvider(adapter));
    registrations.add(languageFeatures.registerReferenceProvider(adapter));
    registrations.add(languageFeatures.registerCallHierarchyProvider(adapter));
    registrations.add(languageFeatures.registerTypeHierarchyProvider(adapter));
    registrations.add(languageFeatures.registerWorkspaceSymbolProvider(new AppServerWorkspaceSymbolProvider(api, workspace)));
    registrations.add(languageFeatures.registerRenameProvider(adapter));
    registrations.add(languageFeatures.registerCodeActionProvider(adapter));
    registrations.add(languageFeatures.registerFormattingProvider(adapter));
    registrations.add(languageFeatures.registerParameterHintsProvider(adapter));
    registrations.add(languageFeatures.registerInlayHintsProvider(adapter));
    registrations.add(languageFeatures.registerLinkedEditingProvider(adapter));
  }
}

class AppServerLanguageProvider implements LanguageCompletionProvider, LanguageHoverProvider, LanguageDeclarationProvider, LanguageDefinitionProvider, LanguageImplementationProvider, LanguageTypeDefinitionProvider, LanguageReferenceProvider, LanguageCallHierarchyProvider, LanguageTypeHierarchyProvider, LanguageRenameProvider, LanguageCodeActionProvider, LanguageFormattingProvider, LanguageParameterHintsProvider, LanguageInlayHintsProvider, LanguageLinkedEditingProvider {
  readonly languageIds = APP_SERVER_LANGUAGE_IDS;
  readonly providerId = "zeta.appServer.language";
  readonly id = "zeta.appServer.completions";
  readonly triggerCharacters = Object.freeze([".", ":", "<", "\"", "'", "/", "@", "#"]);

  constructor(private readonly api: ILanguageApi, private readonly workspace: IWorkspaceContextService) {}

  async provideHover(request: LanguageHoverRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageSnapshotDocument(root, request);
    if (!document) return undefined;
    const result = await this.api.hover({ document, position: dtoPosition(request.position) });
    if (result.revision !== request.snapshot.version || !result.contents) return undefined;
    return Object.freeze({ ...(result.range ? { range: range(result.range) } : {}), contents: Object.freeze([result.contents]) });
  }

  async provideCompletions(request: LanguageCompletionProviderRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageCompletionDocument(root, request);
    if (!document) return undefined;
    const result = await this.api.completions({
      document,
      position: dtoPosition(request.position),
      triggerKind: request.context.kind === LanguageCompletionTriggerKind.Invoke ? "invoke" : request.context.kind === LanguageCompletionTriggerKind.TriggerCharacter ? "triggerCharacter" : "incompleteRefresh",
      triggerCharacter: request.context.kind === LanguageCompletionTriggerKind.TriggerCharacter ? request.context.triggerCharacter : null,
    });
    if (result.revision !== request.snapshot.version) return undefined;
    return Object.freeze({
      isIncomplete: result.isIncomplete,
      items: Object.freeze(result.items.map((item, index) => Object.freeze({
        id: `${request.requestId}:${index}:${item.label}`,
        label: item.label,
        kind: completionKind(item.kind),
        range: range(item.range),
        insertText: item.insertText,
        ...(item.insertTextFormat === "snippet" ? { insertTextFormat: LanguageCompletionInsertTextFormat.Snippet } : {}),
        ...(item.detail ? { detail: item.detail } : {}),
        ...(item.documentation ? { documentation: item.documentation } : {}),
        ...(item.filterText ? { filterText: item.filterText } : {}),
        ...(item.sortText ? { sortText: item.sortText } : {}),
        ...(item.preselect === null ? {} : { preselect: item.preselect }),
        ...(item.commitCharacters.length === 0 ? {} : { commitCharacters: Object.freeze(item.commitCharacters) }),
      }))),
    });
  }

  provideDeclaration(request: LanguageLocationRequest): Promise<readonly LanguageLocation[]> { return this.request("declaration", request, true); }
  provideDefinition(request: LanguageLocationRequest): Promise<readonly LanguageLocation[]> { return this.request("definition", request, true); }
  provideImplementation(request: LanguageLocationRequest): Promise<readonly LanguageLocation[]> { return this.request("implementation", request, true); }
  provideTypeDefinition(request: LanguageLocationRequest): Promise<readonly LanguageLocation[]> { return this.request("typeDefinition", request, true); }
  provideReferences(request: LanguageReferenceRequest): Promise<readonly LanguageLocation[]> { return this.request("references", request, request.includeDeclaration); }
  prepareCallHierarchy(request: LanguageHierarchyRequest): Promise<readonly LanguageHierarchyItem[]> { return this.prepareHierarchy("prepareCall", request); }
  prepareTypeHierarchy(request: LanguageHierarchyRequest): Promise<readonly LanguageHierarchyItem[]> { return this.prepareHierarchy("prepareType", request); }
  provideIncomingCalls(request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageCallHierarchyEntry[]> { return this.followCallHierarchy("incomingCalls", request); }
  provideOutgoingCalls(request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageCallHierarchyEntry[]> { return this.followCallHierarchy("outgoingCalls", request); }
  provideSupertypes(request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageHierarchyItem[]> { return this.followTypeHierarchy("supertypes", request); }
  provideSubtypes(request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageHierarchyItem[]> { return this.followTypeHierarchy("subtypes", request); }
  async prepareRename(request: LanguageRenameRequest): Promise<{ readonly range: TextRange; readonly placeholder: string } | undefined> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return undefined;
    const result = await this.api.prepareRename({ document, position: dtoPosition(request.position) });
    return result.preparation ? Object.freeze({ range: range(result.preparation.range), placeholder: result.preparation.placeholder }) : undefined;
  }
  async provideRenameEdits(request: LanguageRenameRequest) {
    if (!request.newName) throw new Error("Rename request requires a new name");
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) throw new Error("Rename is unavailable because this file is too large for App Server language synchronization");
    return workspaceEdit(root, await this.api.rename({ document, position: dtoPosition(request.position), newName: request.newName }));
  }
  async provideCodeActions(request: LanguageCodeActionRequest): Promise<readonly LanguageCodeAction[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.codeActions({
      document,
      range: dtoRange(request.range),
      diagnostics: request.diagnostics.map(diagnostic => ({ range: dtoRange(diagnostic.range), severity: diagnosticSeverity(diagnostic.severity), message: diagnostic.message, code: diagnostic.code ?? null, source: diagnostic.source ?? null })),
      only: [...(request.only ?? [])],
    });
    return Object.freeze(result.actions.map(action => codeAction(root, action)));
  }
  async resolveCodeAction(action: LanguageCodeAction, request: LanguageCodeActionRequest): Promise<LanguageCodeAction> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    return document ? codeAction(root, await this.api.resolveCodeAction({ document, providerData: action.data })) : action;
  }
  async provideDocumentFormattingEdits(request: LanguageFormattingRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageFormattingDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.formatDocument({ document, options: formattingOptions(request) });
    return result.revision === request.snapshot.version ? formattingEdits(result.edits) : Object.freeze([]);
  }
  async provideRangeFormattingEdits(request: LanguageFormattingRequest) {
    if (!request.range) return Object.freeze([]);
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageFormattingDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.formatRange({ document, range: dtoRange(request.range), options: formattingOptions(request) });
    return result.revision === request.snapshot.version ? formattingEdits(result.edits) : Object.freeze([]);
  }
  async provideParameterHints(request: LanguageParameterHintsRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageParameterHintsDocument(root, request);
    if (!document) return undefined;
    const result = await this.api.signatureHelp({
      document,
      position: dtoPosition(request.position),
      triggerKind: request.context.kind,
      triggerCharacter: request.context.kind === "triggerCharacter" ? request.context.triggerCharacter : null,
    });
    if (result.revision !== request.snapshot.version || result.signatures.length === 0) return undefined;
    return Object.freeze({
      signatures: Object.freeze(result.signatures.map(signature => Object.freeze({
        label: signature.label,
        ...(signature.documentation ? { documentation: signature.documentation } : {}),
        parameters: Object.freeze(signature.parameters.map(parameter => Object.freeze({ label: parameter.label, ...(parameter.documentation ? { documentation: parameter.documentation } : {}) }))),
        ...(signature.activeParameter === null ? {} : { activeParameter: signature.activeParameter }),
      }))),
      ...(result.activeSignature === null ? {} : { activeSignature: result.activeSignature }),
    });
  }
  async provideInlayHints(request: LanguageInlayHintsRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageInlayHintsDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.inlayHints({ document, range: dtoRange(request.range) });
    if (result.revision !== request.snapshot.version) return Object.freeze([]);
    return Object.freeze(result.hints.map(hint => Object.freeze({
      position: TextPosition.at(hint.position.lineIndex, hint.position.columnIndex),
      label: hint.label,
      kind: hint.kind,
      ...(hint.tooltip ? { tooltip: hint.tooltip } : {}),
      paddingLeft: hint.paddingLeft,
      paddingRight: hint.paddingRight,
    })));
  }
  async provideLinkedEditingRanges(request: LanguageLinkedEditingRequest) {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageLinkedEditingDocument(root, request);
    if (!document) return undefined;
    const result = await this.api.linkedEditingRanges({ document, position: dtoPosition(request.position) });
    if (result.revision !== request.snapshot.version || result.ranges.length < 2) return undefined;
    let wordPattern: RegExp | undefined;
    if (result.wordPattern) {
      try { wordPattern = new RegExp(result.wordPattern, "u"); } catch { wordPattern = undefined; }
    }
    return Object.freeze({ ranges: Object.freeze(result.ranges.map(value => range(value))), ...(wordPattern ? { wordPattern } : {}) });
  }

  private async request(kind: LocationKind, request: LanguageLocationRequest, includeDeclaration: boolean): Promise<readonly LanguageLocation[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.locations({
      document,
      position: {
        lineIndex: request.position.lineIndex,
        columnIndex: request.position.columnIndex,
      },
      kind,
      includeDeclaration,
    });
    if (result.revision !== request.snapshot.version) return Object.freeze([]);
    return Object.freeze(result.locations.map(location => {
      const resource = workspaceResource(root, location.path);
      return Object.freeze({
        resource,
        range: range(location.range),
        selectionRange: range(location.selectionRange),
      });
    }));
  }

  private async prepareHierarchy(kind: "prepareCall" | "prepareType", request: LanguageHierarchyRequest): Promise<readonly LanguageHierarchyItem[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.hierarchy({ document, kind, position: dtoPosition(request.position), item: null });
    if (result.revision !== request.snapshot.version) return Object.freeze([]);
    return Object.freeze(result.entries.map(entry => hierarchyItem(root, entry.item)));
  }

  private async followCallHierarchy(kind: "incomingCalls" | "outgoingCalls", request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageCallHierarchyEntry[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.hierarchy({ document, kind, position: null, item: hierarchyItemDto(root, request.item) });
    if (result.revision !== request.snapshot.version) return Object.freeze([]);
    return Object.freeze(result.entries.map(entry => Object.freeze({ item: hierarchyItem(root, entry.item), ...(entry.fromPath ? { fromResource: workspaceResource(root, entry.fromPath) } : {}), fromRanges: Object.freeze(entry.fromRanges.map(range)) })));
  }

  private async followTypeHierarchy(kind: "supertypes" | "subtypes", request: LanguageHierarchyFollowupRequest): Promise<readonly LanguageHierarchyItem[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const document = languageDocument(root, request);
    if (!document) return Object.freeze([]);
    const result = await this.api.hierarchy({ document, kind, position: null, item: hierarchyItemDto(root, request.item) });
    if (result.revision !== request.snapshot.version) return Object.freeze([]);
    return Object.freeze(result.entries.map(entry => hierarchyItem(root, entry.item)));
  }
}

class AppServerWorkspaceSymbolProvider implements LanguageWorkspaceSymbolProvider {
  readonly languageIds = Object.freeze(["*"]);
  readonly providerId = "zeta.appServer.workspaceSymbols";

  constructor(private readonly api: ILanguageApi, private readonly workspace: IWorkspaceContextService) {}

  async provideWorkspaceSymbols(query: string, signal: AbortSignal): Promise<readonly LanguageWorkspaceSymbol[]> {
    const root = singleWorkspaceRoot(this.workspace);
    const responses = await Promise.all(APP_SERVER_LANGUAGE_IDS.map(async languageId => {
      if (signal.aborted) return [];
      try { return (await this.api.workspaceSymbols({ languageId, query })).symbols; } catch { return []; }
    }));
    if (signal.aborted) return Object.freeze([]);
    const seen = new Set<string>();
    return Object.freeze(responses.flat().flatMap(symbol => {
      const resource = workspaceResource(root, symbol.path);
      const symbolRange = range(symbol.range);
      const key = `${resource.toString()}\0${symbol.name}\0${symbolRange.start.lineIndex}:${symbolRange.start.columnIndex}`;
      if (seen.has(key)) return [];
      seen.add(key);
      return [Object.freeze({ name: symbol.name, kind: symbol.symbolKind, resource, range: symbolRange, ...(symbol.containerName ? { containerName: symbol.containerName } : {}) })];
    }));
  }
}

function languageDocument(root: URI, request: LanguageLocationRequest | LanguageHierarchyRequest | LanguageHierarchyFollowupRequest | LanguageRenameRequest | LanguageCodeActionRequest) {
  if (request.model.largeFile.tooLargeForSynchronization) return undefined;
  const text = request.snapshot.getText();
  if (new TextEncoder().encode(text).byteLength > APP_SERVER_LANGUAGE_DOCUMENT_MAX_BYTES) return undefined;
  return { path: workspaceRelativePath(root, request.resource), languageId: request.languageId, revision: request.snapshot.version, text };
}

function languageCompletionDocument(root: URI, request: LanguageCompletionProviderRequest) {
  if (!request.resource) return undefined;
  return languageSnapshotDocument(root, { resource: request.resource, languageId: request.languageId, snapshot: request.snapshot });
}

function languageSnapshotDocument(root: URI, request: { readonly resource?: URI; readonly languageId: string; readonly snapshot: { readonly version: number; getText(): string } }) {
  if (!request.resource) return undefined;
  const text = request.snapshot.getText();
  if (new TextEncoder().encode(text).byteLength > APP_SERVER_LANGUAGE_DOCUMENT_MAX_BYTES) return undefined;
  return { path: workspaceRelativePath(root, request.resource), languageId: request.languageId, revision: request.snapshot.version, text };
}

function languageFormattingDocument(root: URI, request: LanguageFormattingRequest) {
  if (!request.resource || request.model.largeFile.tooLargeForSynchronization) return undefined;
  return languageSnapshotDocument(root, request);
}

function languageParameterHintsDocument(root: URI, request: LanguageParameterHintsRequest) {
  if (!request.resource || request.model.largeFile.tooLargeForSynchronization) return undefined;
  return languageSnapshotDocument(root, request);
}

function languageInlayHintsDocument(root: URI, request: LanguageInlayHintsRequest) {
  if (!request.resource || request.model.largeFile.tooLargeForSynchronization) return undefined;
  return languageSnapshotDocument(root, request);
}

function languageLinkedEditingDocument(root: URI, request: LanguageLinkedEditingRequest) {
  if (!request.resource || request.model.largeFile.tooLargeForSynchronization) return undefined;
  return languageSnapshotDocument(root, request);
}

function formattingOptions(request: LanguageFormattingRequest) {
  return { tabSize: request.options.tabSize, insertSpaces: request.options.insertSpaces, trimTrailingWhitespace: request.options.trimTrailingWhitespace ?? null };
}

function formattingEdits(edits: readonly { readonly range: { readonly start: { readonly lineIndex: number; readonly columnIndex: number }; readonly end: { readonly lineIndex: number; readonly columnIndex: number } }; readonly newText: string }[]) {
  return Object.freeze(edits.map(edit => Object.freeze({ range: range(edit.range), text: edit.newText })));
}

function completionKind(kind: LanguageCompletionItemKindDto): LanguageCompletionItemKind {
  switch (kind) {
    case "method": return LanguageCompletionItemKind.Method;
    case "function": return LanguageCompletionItemKind.Function;
    case "constructor": return LanguageCompletionItemKind.Constructor;
    case "field": return LanguageCompletionItemKind.Field;
    case "variable": return LanguageCompletionItemKind.Variable;
    case "class": return LanguageCompletionItemKind.Class;
    case "interface": return LanguageCompletionItemKind.Interface;
    case "module": return LanguageCompletionItemKind.Module;
    case "property": return LanguageCompletionItemKind.Property;
    case "unit": return LanguageCompletionItemKind.Unit;
    case "value": return LanguageCompletionItemKind.Value;
    case "enum": return LanguageCompletionItemKind.Enum;
    case "keyword": return LanguageCompletionItemKind.Keyword;
    case "snippet": return LanguageCompletionItemKind.Snippet;
    case "file": return LanguageCompletionItemKind.File;
    case "folder": return LanguageCompletionItemKind.Folder;
    case "reference": return LanguageCompletionItemKind.Reference;
    case "typeParameter": return LanguageCompletionItemKind.TypeParameter;
    case "text": return LanguageCompletionItemKind.Text;
  }
}

function dtoPosition(position: TextPosition): { readonly lineIndex: number; readonly columnIndex: number } { return { lineIndex: position.lineIndex, columnIndex: position.columnIndex }; }

function hierarchyItem(root: URI, item: LanguageHierarchyItemDto): LanguageHierarchyItem {
  return Object.freeze({ name: item.name, symbolKind: item.symbolKind, ...(item.detail ? { detail: item.detail } : {}), resource: workspaceResource(root, item.path), range: range(item.range), selectionRange: range(item.selectionRange), ...(item.data === undefined ? {} : { data: item.data }) });
}

function hierarchyItemDto(root: URI, item: LanguageHierarchyItem): LanguageHierarchyItemDto {
  return { name: item.name, symbolKind: item.symbolKind, detail: item.detail ?? null, path: workspaceRelativePath(root, item.resource), range: dtoRange(item.range), selectionRange: dtoRange(item.selectionRange), data: item.data };
}

function dtoRange(value: TextRange) { return { start: dtoPosition(value.start), end: dtoPosition(value.end) }; }

function workspaceEdit(root: URI, edit: LanguageWorkspaceEditDto) {
  return Object.freeze({ entries: Object.freeze(edit.entries.map(entry => {
    switch (entry.kind) {
      case "textDocument": return Object.freeze({ kind: entry.kind, resource: workspaceResource(root, entry.document.path), expectedText: entry.document.expectedText, edits: Object.freeze(entry.document.edits.map(edit => Object.freeze({ range: range(edit.range), text: edit.newText }))) });
      case "create": return Object.freeze({ kind: entry.kind, resource: workspaceResource(root, entry.path), existing: entry.existing });
      case "rename": return Object.freeze({ kind: entry.kind, source: workspaceResource(root, entry.source), target: workspaceResource(root, entry.target), existing: entry.existing });
      case "delete": return Object.freeze({ kind: entry.kind, resource: workspaceResource(root, entry.path), missing: entry.missing, mode: entry.mode });
    }
  })) });
}

function codeAction(root: URI, action: LanguageCodeActionDto): LanguageCodeAction {
  return Object.freeze({ title: action.title, ...(action.kind ? { kind: action.kind } : {}), isPreferred: action.isPreferred, ...(action.disabledReason ? { disabledReason: action.disabledReason } : {}), ...(action.edit ? { edit: workspaceEdit(root, action.edit) } : {}), data: action.providerData });
}

function diagnosticSeverity(severity: LanguageDiagnosticSeverity): "error" | "warning" | "information" | "hint" {
  return severity;
}

function singleWorkspaceRoot(workspace: IWorkspaceContextService): URI {
  const folders = workspace.getWorkspace().folders;
  if (folders.length !== 1) throw new Error("Language service requires one workspace folder");
  return folders[0]!.uri;
}

function workspaceResource(root: URI, relativePath: string): URI {
  const resource = workspaceResourceFromPath(root, relativePath);
  if (!resource) throw new Error("Language service returned an invalid workspace path");
  return resource;
}

function range(value: { readonly start: { readonly lineIndex: number; readonly columnIndex: number }; readonly end: { readonly lineIndex: number; readonly columnIndex: number } }): TextRange {
  return TextRange.from(TextPosition.at(value.start.lineIndex, value.start.columnIndex), TextPosition.at(value.end.lineIndex, value.end.columnIndex));
}
