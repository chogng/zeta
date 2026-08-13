import type { LanguageCloseParams, LanguageCodeActionDto, LanguageCodeActionsParams, LanguageCodeActionsResult, LanguageCompletionsParams, LanguageCompletionsResult, LanguageDocumentFormattingParams, LanguageFormattingResult, LanguageHierarchyParams, LanguageHierarchyResultDto, LanguageHoverParams, LanguageHoverResult, LanguageInlayHintsParams, LanguageInlayHintsResult, LanguageLinkedEditingRangesParams, LanguageLinkedEditingRangesResult, LanguageLocationsParams, LanguageLocationsResult, LanguagePrepareRenameParams, LanguagePrepareRenameResult, LanguageRangeFormattingParams, LanguageRenameParams, LanguageResolveCodeActionParams, LanguageSignatureHelpParams, LanguageSignatureHelpResult, LanguageSynchronizeParams, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsParams, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";

/** Transport-neutral entry point for workspace language-server locations. */
export interface ILanguageApi {
  synchronize(params: LanguageSynchronizeParams): Promise<void>;
  close(params: LanguageCloseParams): Promise<void>;
  hover(params: LanguageHoverParams): Promise<LanguageHoverResult>;
  completions(params: LanguageCompletionsParams): Promise<LanguageCompletionsResult>;
  formatDocument(params: LanguageDocumentFormattingParams): Promise<LanguageFormattingResult>;
  formatRange(params: LanguageRangeFormattingParams): Promise<LanguageFormattingResult>;
  signatureHelp(params: LanguageSignatureHelpParams): Promise<LanguageSignatureHelpResult>;
  inlayHints(params: LanguageInlayHintsParams): Promise<LanguageInlayHintsResult>;
  linkedEditingRanges(params: LanguageLinkedEditingRangesParams): Promise<LanguageLinkedEditingRangesResult>;
  locations(params: LanguageLocationsParams): Promise<LanguageLocationsResult>;
  hierarchy(params: LanguageHierarchyParams): Promise<LanguageHierarchyResultDto>;
  workspaceSymbols(params: LanguageWorkspaceSymbolsParams): Promise<LanguageWorkspaceSymbolsResult>;
  prepareRename(params: LanguagePrepareRenameParams): Promise<LanguagePrepareRenameResult>;
  rename(params: LanguageRenameParams): Promise<LanguageWorkspaceEditDto>;
  codeActions(params: LanguageCodeActionsParams): Promise<LanguageCodeActionsResult>;
  resolveCodeAction(params: LanguageResolveCodeActionParams): Promise<LanguageCodeActionDto>;
}
