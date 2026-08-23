import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest, voidResult } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createDisconnectedLanguageApi(unavailable: UnavailableOperation): ILanguageApi {
  return { synchronize: () => unavailable("language.synchronize"), close: () => unavailable("language.close"), hover: () => unavailable("language.hover"), completions: () => unavailable("language.completions"), resolveCompletion: () => unavailable("language.resolveCompletion"), executeCommand: () => unavailable("language.executeCommand"), documentDiagnostics: () => unavailable("language.documentDiagnostics"), workspaceDiagnostics: () => unavailable("language.workspaceDiagnostics"), formatDocument: () => unavailable("language.formatDocument"), formatRange: () => unavailable("language.formatRange"), signatureHelp: () => unavailable("language.signatureHelp"), inlayHints: () => unavailable("language.inlayHints"), linkedEditingRanges: () => unavailable("language.linkedEditingRanges"), semanticTokens: () => unavailable("language.semanticTokens"), documentSymbols: () => unavailable("language.documentSymbols"), codeLenses: () => unavailable("language.codeLenses"), resolveCodeLens: () => unavailable("language.resolveCodeLens"), documentLinks: () => unavailable("language.documentLinks"), resolveDocumentLink: () => unavailable("language.resolveDocumentLink"), documentColors: () => unavailable("language.documentColors"), colorPresentations: () => unavailable("language.colorPresentations"), foldingRanges: () => unavailable("language.foldingRanges"), locations: () => unavailable("language.locations"), hierarchy: () => unavailable("language.hierarchy"), workspaceSymbols: () => unavailable("language.workspaceSymbols"), prepareRename: () => unavailable("language.prepareRename"), rename: () => unavailable("language.rename"), codeActions: () => unavailable("language.codeActions"), resolveCodeAction: () => unavailable("language.resolveCodeAction") };
}

export function createViteDevLanguageApi(connection: ViteDevAppServerConnection): ILanguageApi {
  return {
    synchronize: params => voidResult(viteDevRequest(connection, "language/synchronize", params)),
    close: params => voidResult(viteDevRequest(connection, "language/close", params)),
    hover: params => viteDevRequest(connection, "language/hover", params),
    completions: params => viteDevRequest(connection, "language/completions", params),
    resolveCompletion: params => viteDevRequest(connection, "language/resolveCompletion", params),
    executeCommand: params => voidResult(viteDevRequest(connection, "language/executeCommand", params)),
    documentDiagnostics: params => viteDevRequest(connection, "language/documentDiagnostics", params),
    workspaceDiagnostics: params => viteDevRequest(connection, "language/workspaceDiagnostics", params),
    formatDocument: params => viteDevRequest(connection, "language/formatDocument", params),
    formatRange: params => viteDevRequest(connection, "language/formatRange", params),
    signatureHelp: params => viteDevRequest(connection, "language/signatureHelp", params),
    inlayHints: params => viteDevRequest(connection, "language/inlayHints", params),
    linkedEditingRanges: params => viteDevRequest(connection, "language/linkedEditingRanges", params),
    semanticTokens: params => viteDevRequest(connection, "language/semanticTokens", params),
    documentSymbols: params => viteDevRequest(connection, "language/documentSymbols", params),
    codeLenses: params => viteDevRequest(connection, "language/codeLenses", params),
    resolveCodeLens: params => viteDevRequest(connection, "language/resolveCodeLens", params),
    documentLinks: params => viteDevRequest(connection, "language/documentLinks", params),
    resolveDocumentLink: params => viteDevRequest(connection, "language/resolveDocumentLink", params),
    documentColors: params => viteDevRequest(connection, "language/documentColors", params),
    colorPresentations: params => viteDevRequest(connection, "language/colorPresentations", params),
    foldingRanges: params => viteDevRequest(connection, "language/foldingRanges", params),
    locations: params => viteDevRequest(connection, "language/locations", params),
    hierarchy: params => viteDevRequest(connection, "language/hierarchy", params),
    workspaceSymbols: params => viteDevRequest(connection, "language/workspaceSymbols", params),
    prepareRename: params => viteDevRequest(connection, "language/prepareRename", params),
    rename: params => viteDevRequest(connection, "language/rename", params),
    codeActions: params => viteDevRequest(connection, "language/codeActions", params),
    resolveCodeAction: params => viteDevRequest(connection, "language/resolveCodeAction", params),
  };
}
