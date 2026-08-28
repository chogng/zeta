import type { LanguageCodeActionDto, LanguageCodeActionsResult, LanguageCodeLensesResult, LanguageColorPresentationsResult, LanguageCompletionDetailsResult, LanguageCompletionsResult, LanguageDocumentColorsResult, LanguageDocumentDiagnosticsResult, LanguageDocumentLinksResult, LanguageDocumentSymbolsResult, LanguageFoldingRangesResult, LanguageFormattingResult, LanguageHierarchyResultDto, LanguageHoverResult, LanguageInlayHintsResult, LanguageLinkedEditingRangesResult, LanguageLocationsResult, LanguagePrepareRenameResult, LanguageSemanticTokensResult, LanguageSignatureHelpResult, LanguageWorkspaceDiagnosticsResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";
import { CancellationError } from "../../../base/common/errors.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ILanguageApi, LanguageRequestOptions } from "../common/languageApi.js";

let nextLanguageRequestId = 1;

/** Sends one query through a renderer-scoped IPC envelope that can be cancelled by the caller. */
export function invokeLanguageRequest<TResult>(channel: string, params: unknown, options?: LanguageRequestOptions): Promise<TResult> {
	const signal = options?.signal;
	if (signal?.aborted) return Promise.reject(new CancellationError("Language request cancelled", signal.reason));
	const requestId = `language-${nextLanguageRequestId++}`;
	const pending = invoke<TResult>(channel, { requestId, params });
	if (!signal) return pending;
	return new Promise<TResult>((resolve, reject) => {
		let settled = false;
		const cleanup = (): void => signal.removeEventListener("abort", abort);
		const abort = (): void => {
			if (settled) return;
			settled = true;
			cleanup();
			void invoke<void>("zeta:language:cancel", { requestId }).catch(() => undefined);
			reject(new CancellationError("Language request cancelled", signal.reason));
		};
		signal.addEventListener("abort", abort, { once: true });
		pending.then(
			value => {
				if (settled) return;
				settled = true;
				cleanup();
				resolve(value);
			},
			error => {
				if (settled) return;
				settled = true;
				cleanup();
				reject(error);
			},
		);
	});
}

export function createLanguageApi(): ILanguageApi {
	return {
		synchronize: params => invoke<void>("zeta:language:synchronize", params),
		close: params => invoke<void>("zeta:language:close", params),
		hover: (params, options) => invokeLanguageRequest<LanguageHoverResult>("zeta:language:hover", params, options),
		completions: (params, options) => invokeLanguageRequest<LanguageCompletionsResult>("zeta:language:completions", params, options),
		resolveCompletion: (params, options) => invokeLanguageRequest<LanguageCompletionDetailsResult>("zeta:language:resolveCompletion", params, options),
		executeCommand: params => invoke<void>("zeta:language:executeCommand", params),
		documentDiagnostics: (params, options) => invokeLanguageRequest<LanguageDocumentDiagnosticsResult>("zeta:language:documentDiagnostics", params, options),
		workspaceDiagnostics: (params, options) => invokeLanguageRequest<LanguageWorkspaceDiagnosticsResult>("zeta:language:workspaceDiagnostics", params, options),
		formatDocument: (params, options) => invokeLanguageRequest<LanguageFormattingResult>("zeta:language:formatDocument", params, options),
		formatRange: (params, options) => invokeLanguageRequest<LanguageFormattingResult>("zeta:language:formatRange", params, options),
		signatureHelp: (params, options) => invokeLanguageRequest<LanguageSignatureHelpResult>("zeta:language:signatureHelp", params, options),
		inlayHints: (params, options) => invokeLanguageRequest<LanguageInlayHintsResult>("zeta:language:inlayHints", params, options),
		linkedEditingRanges: (params, options) => invokeLanguageRequest<LanguageLinkedEditingRangesResult>("zeta:language:linkedEditingRanges", params, options),
		semanticTokens: (params, options) => invokeLanguageRequest<LanguageSemanticTokensResult>("zeta:language:semanticTokens", params, options),
		documentSymbols: (params, options) => invokeLanguageRequest<LanguageDocumentSymbolsResult>("zeta:language:documentSymbols", params, options),
		codeLenses: (params, options) => invokeLanguageRequest<LanguageCodeLensesResult>("zeta:language:codeLenses", params, options),
		resolveCodeLens: (params, options) => invokeLanguageRequest<LanguageCodeLensesResult>("zeta:language:resolveCodeLens", params, options),
		documentLinks: (params, options) => invokeLanguageRequest<LanguageDocumentLinksResult>("zeta:language:documentLinks", params, options),
		resolveDocumentLink: (params, options) => invokeLanguageRequest<LanguageDocumentLinksResult>("zeta:language:resolveDocumentLink", params, options),
		documentColors: (params, options) => invokeLanguageRequest<LanguageDocumentColorsResult>("zeta:language:documentColors", params, options),
		colorPresentations: (params, options) => invokeLanguageRequest<LanguageColorPresentationsResult>("zeta:language:colorPresentations", params, options),
		foldingRanges: (params, options) => invokeLanguageRequest<LanguageFoldingRangesResult>("zeta:language:foldingRanges", params, options),
		locations: (params, options) => invokeLanguageRequest<LanguageLocationsResult>("zeta:language:locations", params, options),
		hierarchy: (params, options) => invokeLanguageRequest<LanguageHierarchyResultDto>("zeta:language:hierarchy", params, options),
		workspaceSymbols: (params, options) => invokeLanguageRequest<LanguageWorkspaceSymbolsResult>("zeta:language:workspaceSymbols", params, options),
		prepareRename: (params, options) => invokeLanguageRequest<LanguagePrepareRenameResult>("zeta:language:prepareRename", params, options),
		rename: (params, options) => invokeLanguageRequest<LanguageWorkspaceEditDto>("zeta:language:rename", params, options),
		codeActions: (params, options) => invokeLanguageRequest<LanguageCodeActionsResult>("zeta:language:codeActions", params, options),
		resolveCodeAction: (params, options) => invokeLanguageRequest<LanguageCodeActionDto>("zeta:language:resolveCodeAction", params, options),
	};
}
