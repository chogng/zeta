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
		hover: (params, options) => viteDevRequest(connection, "language/hover", params, options),
		completions: (params, options) => viteDevRequest(connection, "language/completions", params, options),
		resolveCompletion: (params, options) => viteDevRequest(connection, "language/resolveCompletion", params, options),
		executeCommand: params => voidResult(viteDevRequest(connection, "language/executeCommand", params)),
		documentDiagnostics: (params, options) => viteDevRequest(connection, "language/documentDiagnostics", params, options),
		workspaceDiagnostics: (params, options) => viteDevRequest(connection, "language/workspaceDiagnostics", params, options),
		formatDocument: (params, options) => viteDevRequest(connection, "language/formatDocument", params, options),
		formatRange: (params, options) => viteDevRequest(connection, "language/formatRange", params, options),
		signatureHelp: (params, options) => viteDevRequest(connection, "language/signatureHelp", params, options),
		inlayHints: (params, options) => viteDevRequest(connection, "language/inlayHints", params, options),
		linkedEditingRanges: (params, options) => viteDevRequest(connection, "language/linkedEditingRanges", params, options),
		semanticTokens: (params, options) => viteDevRequest(connection, "language/semanticTokens", params, options),
		documentSymbols: (params, options) => viteDevRequest(connection, "language/documentSymbols", params, options),
		codeLenses: (params, options) => viteDevRequest(connection, "language/codeLenses", params, options),
		resolveCodeLens: (params, options) => viteDevRequest(connection, "language/resolveCodeLens", params, options),
		documentLinks: (params, options) => viteDevRequest(connection, "language/documentLinks", params, options),
		resolveDocumentLink: (params, options) => viteDevRequest(connection, "language/resolveDocumentLink", params, options),
		documentColors: (params, options) => viteDevRequest(connection, "language/documentColors", params, options),
		colorPresentations: (params, options) => viteDevRequest(connection, "language/colorPresentations", params, options),
		foldingRanges: (params, options) => viteDevRequest(connection, "language/foldingRanges", params, options),
		locations: (params, options) => viteDevRequest(connection, "language/locations", params, options),
		hierarchy: (params, options) => viteDevRequest(connection, "language/hierarchy", params, options),
		workspaceSymbols: (params, options) => viteDevRequest(connection, "language/workspaceSymbols", params, options),
		prepareRename: (params, options) => viteDevRequest(connection, "language/prepareRename", params, options),
		rename: (params, options) => viteDevRequest(connection, "language/rename", params, options),
		codeActions: (params, options) => viteDevRequest(connection, "language/codeActions", params, options),
		resolveCodeAction: (params, options) => viteDevRequest(connection, "language/resolveCodeAction", params, options),
	};
}
