import type { LanguageCloseParams, LanguageCodeActionDto, LanguageCodeActionsParams, LanguageCodeActionsResult, LanguageCodeLensesResult, LanguageColorPresentationsParams, LanguageColorPresentationsResult, LanguageCompletionDetailsResult, LanguageCompletionsParams, LanguageCompletionsResult, LanguageDocumentColorsResult, LanguageDocumentDiagnosticsParams, LanguageDocumentDiagnosticsResult, LanguageDocumentFeaturesParams, LanguageDocumentFormattingParams, LanguageDocumentLinksResult, LanguageDocumentSymbolsResult, LanguageExecuteCommandParams, LanguageFoldingRangesResult, LanguageFormattingResult, LanguageHierarchyParams, LanguageHierarchyResultDto, LanguageHoverParams, LanguageHoverResult, LanguageInlayHintsParams, LanguageInlayHintsResult, LanguageLinkedEditingRangesParams, LanguageLinkedEditingRangesResult, LanguageLocationsParams, LanguageLocationsResult, LanguagePrepareRenameParams, LanguagePrepareRenameResult, LanguageRangeFormattingParams, LanguageRenameParams, LanguageResolveCodeActionParams, LanguageResolveCodeLensParams, LanguageResolveCompletionParams, LanguageResolveDocumentLinkParams, LanguageSemanticTokensParams, LanguageSemanticTokensResult, LanguageSignatureHelpParams, LanguageSignatureHelpResult, LanguageSynchronizeParams, LanguageWorkspaceDiagnosticsParams, LanguageWorkspaceDiagnosticsResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsParams, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";

/** Transport-neutral entry point for workspace language-server locations. */
export interface LanguageRequestOptions {
	readonly signal?: AbortSignal;
}

export interface ILanguageApi {
	synchronize(params: LanguageSynchronizeParams): Promise<void>;
	close(params: LanguageCloseParams): Promise<void>;
	hover(params: LanguageHoverParams, options?: LanguageRequestOptions): Promise<LanguageHoverResult>;
	completions(params: LanguageCompletionsParams, options?: LanguageRequestOptions): Promise<LanguageCompletionsResult>;
	resolveCompletion(params: LanguageResolveCompletionParams, options?: LanguageRequestOptions): Promise<LanguageCompletionDetailsResult>;
	executeCommand(params: LanguageExecuteCommandParams): Promise<void>;
	documentDiagnostics(params: LanguageDocumentDiagnosticsParams, options?: LanguageRequestOptions): Promise<LanguageDocumentDiagnosticsResult>;
	workspaceDiagnostics(params: LanguageWorkspaceDiagnosticsParams, options?: LanguageRequestOptions): Promise<LanguageWorkspaceDiagnosticsResult>;
	formatDocument(params: LanguageDocumentFormattingParams, options?: LanguageRequestOptions): Promise<LanguageFormattingResult>;
	formatRange(params: LanguageRangeFormattingParams, options?: LanguageRequestOptions): Promise<LanguageFormattingResult>;
	signatureHelp(params: LanguageSignatureHelpParams, options?: LanguageRequestOptions): Promise<LanguageSignatureHelpResult>;
	inlayHints(params: LanguageInlayHintsParams, options?: LanguageRequestOptions): Promise<LanguageInlayHintsResult>;
	linkedEditingRanges(params: LanguageLinkedEditingRangesParams, options?: LanguageRequestOptions): Promise<LanguageLinkedEditingRangesResult>;
	semanticTokens(params: LanguageSemanticTokensParams, options?: LanguageRequestOptions): Promise<LanguageSemanticTokensResult>;
	documentSymbols(params: LanguageDocumentFeaturesParams, options?: LanguageRequestOptions): Promise<LanguageDocumentSymbolsResult>;
	codeLenses(params: LanguageDocumentFeaturesParams, options?: LanguageRequestOptions): Promise<LanguageCodeLensesResult>;
	resolveCodeLens(params: LanguageResolveCodeLensParams, options?: LanguageRequestOptions): Promise<LanguageCodeLensesResult>;
	documentLinks(params: LanguageDocumentFeaturesParams, options?: LanguageRequestOptions): Promise<LanguageDocumentLinksResult>;
	resolveDocumentLink(params: LanguageResolveDocumentLinkParams, options?: LanguageRequestOptions): Promise<LanguageDocumentLinksResult>;
	documentColors(params: LanguageDocumentFeaturesParams, options?: LanguageRequestOptions): Promise<LanguageDocumentColorsResult>;
	colorPresentations(params: LanguageColorPresentationsParams, options?: LanguageRequestOptions): Promise<LanguageColorPresentationsResult>;
	foldingRanges(params: LanguageDocumentFeaturesParams, options?: LanguageRequestOptions): Promise<LanguageFoldingRangesResult>;
	locations(params: LanguageLocationsParams, options?: LanguageRequestOptions): Promise<LanguageLocationsResult>;
	hierarchy(params: LanguageHierarchyParams, options?: LanguageRequestOptions): Promise<LanguageHierarchyResultDto>;
	workspaceSymbols(params: LanguageWorkspaceSymbolsParams, options?: LanguageRequestOptions): Promise<LanguageWorkspaceSymbolsResult>;
	prepareRename(params: LanguagePrepareRenameParams, options?: LanguageRequestOptions): Promise<LanguagePrepareRenameResult>;
	rename(params: LanguageRenameParams, options?: LanguageRequestOptions): Promise<LanguageWorkspaceEditDto>;
	codeActions(params: LanguageCodeActionsParams, options?: LanguageRequestOptions): Promise<LanguageCodeActionsResult>;
	resolveCodeAction(params: LanguageResolveCodeActionParams, options?: LanguageRequestOptions): Promise<LanguageCodeActionDto>;
}
