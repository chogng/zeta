import type { LanguageCloseParams, LanguageCodeActionDto, LanguageCodeActionsParams, LanguageCodeActionsResult, LanguageCodeLensesResult, LanguageColorPresentationsParams, LanguageColorPresentationsResult, LanguageCompletionDetailsResult, LanguageCompletionsParams, LanguageCompletionsResult, LanguageDocumentColorsResult, LanguageDocumentDiagnosticsParams, LanguageDocumentDiagnosticsResult, LanguageDocumentFeaturesParams, LanguageDocumentFormattingParams, LanguageDocumentLinksResult, LanguageDocumentSymbolsResult, LanguageExecuteCommandParams, LanguageFoldingRangesResult, LanguageFormattingResult, LanguageHierarchyParams, LanguageHierarchyResultDto, LanguageHoverParams, LanguageHoverResult, LanguageInlayHintsParams, LanguageInlayHintsResult, LanguageLinkedEditingRangesParams, LanguageLinkedEditingRangesResult, LanguageLocationsParams, LanguageLocationsResult, LanguageMarketplaceInstallParams, LanguageMarketplaceInstallResult, LanguageMarketplaceListResult, LanguagePrepareRenameParams, LanguagePrepareRenameResult, LanguageRangeFormattingParams, LanguageRenameParams, LanguageResolveCodeActionParams, LanguageResolveCodeLensParams, LanguageResolveCompletionParams, LanguageResolveDocumentLinkParams, LanguageSemanticTokensParams, LanguageSemanticTokensResult, LanguageSignatureHelpParams, LanguageSignatureHelpResult, LanguageSynchronizeParams, LanguageWorkspaceDiagnosticsParams, LanguageWorkspaceDiagnosticsResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsParams, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";

/** Transport-neutral entry point for workspace language-server locations. */
export interface ILanguageApi {
  listMarketplace(): Promise<LanguageMarketplaceListResult>;
  installMarketplace(params: LanguageMarketplaceInstallParams): Promise<LanguageMarketplaceInstallResult>;
  synchronize(params: LanguageSynchronizeParams): Promise<void>;
  close(params: LanguageCloseParams): Promise<void>;
  hover(params: LanguageHoverParams): Promise<LanguageHoverResult>;
  completions(params: LanguageCompletionsParams): Promise<LanguageCompletionsResult>;
  resolveCompletion(params: LanguageResolveCompletionParams): Promise<LanguageCompletionDetailsResult>;
  executeCommand(params: LanguageExecuteCommandParams): Promise<void>;
  documentDiagnostics(params: LanguageDocumentDiagnosticsParams): Promise<LanguageDocumentDiagnosticsResult>;
  workspaceDiagnostics(params: LanguageWorkspaceDiagnosticsParams): Promise<LanguageWorkspaceDiagnosticsResult>;
  formatDocument(params: LanguageDocumentFormattingParams): Promise<LanguageFormattingResult>;
  formatRange(params: LanguageRangeFormattingParams): Promise<LanguageFormattingResult>;
  signatureHelp(params: LanguageSignatureHelpParams): Promise<LanguageSignatureHelpResult>;
  inlayHints(params: LanguageInlayHintsParams): Promise<LanguageInlayHintsResult>;
  linkedEditingRanges(params: LanguageLinkedEditingRangesParams): Promise<LanguageLinkedEditingRangesResult>;
  semanticTokens(params: LanguageSemanticTokensParams): Promise<LanguageSemanticTokensResult>;
  documentSymbols(params: LanguageDocumentFeaturesParams): Promise<LanguageDocumentSymbolsResult>;
  codeLenses(params: LanguageDocumentFeaturesParams): Promise<LanguageCodeLensesResult>;
  resolveCodeLens(params: LanguageResolveCodeLensParams): Promise<LanguageCodeLensesResult>;
  documentLinks(params: LanguageDocumentFeaturesParams): Promise<LanguageDocumentLinksResult>;
  resolveDocumentLink(params: LanguageResolveDocumentLinkParams): Promise<LanguageDocumentLinksResult>;
  documentColors(params: LanguageDocumentFeaturesParams): Promise<LanguageDocumentColorsResult>;
  colorPresentations(params: LanguageColorPresentationsParams): Promise<LanguageColorPresentationsResult>;
  foldingRanges(params: LanguageDocumentFeaturesParams): Promise<LanguageFoldingRangesResult>;
  locations(params: LanguageLocationsParams): Promise<LanguageLocationsResult>;
  hierarchy(params: LanguageHierarchyParams): Promise<LanguageHierarchyResultDto>;
  workspaceSymbols(params: LanguageWorkspaceSymbolsParams): Promise<LanguageWorkspaceSymbolsResult>;
  prepareRename(params: LanguagePrepareRenameParams): Promise<LanguagePrepareRenameResult>;
  rename(params: LanguageRenameParams): Promise<LanguageWorkspaceEditDto>;
  codeActions(params: LanguageCodeActionsParams): Promise<LanguageCodeActionsResult>;
  resolveCodeAction(params: LanguageResolveCodeActionParams): Promise<LanguageCodeActionDto>;
}
