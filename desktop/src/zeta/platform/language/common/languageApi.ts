import type { LanguageCodeActionDto, LanguageCodeActionsParams, LanguageCodeActionsResult, LanguageHierarchyParams, LanguageHierarchyResultDto, LanguageLocationsParams, LanguageLocationsResult, LanguagePrepareRenameParams, LanguagePrepareRenameResult, LanguageRenameParams, LanguageResolveCodeActionParams, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsParams, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";

/** Transport-neutral entry point for workspace language-server locations. */
export interface ILanguageApi {
  locations(params: LanguageLocationsParams): Promise<LanguageLocationsResult>;
  hierarchy(params: LanguageHierarchyParams): Promise<LanguageHierarchyResultDto>;
  workspaceSymbols(params: LanguageWorkspaceSymbolsParams): Promise<LanguageWorkspaceSymbolsResult>;
  prepareRename(params: LanguagePrepareRenameParams): Promise<LanguagePrepareRenameResult>;
  rename(params: LanguageRenameParams): Promise<LanguageWorkspaceEditDto>;
  codeActions(params: LanguageCodeActionsParams): Promise<LanguageCodeActionsResult>;
  resolveCodeAction(params: LanguageResolveCodeActionParams): Promise<LanguageCodeActionDto>;
}
