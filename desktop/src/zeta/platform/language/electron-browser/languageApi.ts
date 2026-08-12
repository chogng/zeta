import type { LanguageCodeActionDto, LanguageCodeActionsResult, LanguageHierarchyResultDto, LanguageLocationsResult, LanguagePrepareRenameResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createLanguageApi(): ILanguageApi {
  return {
    locations: params => invoke<LanguageLocationsResult>("zeta:language:locations", params),
    hierarchy: params => invoke<LanguageHierarchyResultDto>("zeta:language:hierarchy", params),
    workspaceSymbols: params => invoke<LanguageWorkspaceSymbolsResult>("zeta:language:workspaceSymbols", params),
    prepareRename: params => invoke<LanguagePrepareRenameResult>("zeta:language:prepareRename", params),
    rename: params => invoke<LanguageWorkspaceEditDto>("zeta:language:rename", params),
    codeActions: params => invoke<LanguageCodeActionsResult>("zeta:language:codeActions", params),
    resolveCodeAction: params => invoke<LanguageCodeActionDto>("zeta:language:resolveCodeAction", params),
  };
}
