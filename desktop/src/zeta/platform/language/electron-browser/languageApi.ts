import type { LanguageCodeActionDto, LanguageCodeActionsResult, LanguageCompletionsResult, LanguageFormattingResult, LanguageHierarchyResultDto, LanguageHoverResult, LanguageInlayHintsResult, LanguageLinkedEditingRangesResult, LanguageLocationsResult, LanguagePrepareRenameResult, LanguageSignatureHelpResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createLanguageApi(): ILanguageApi {
  return {
    synchronize: params => invoke<void>("zeta:language:synchronize", params),
    close: params => invoke<void>("zeta:language:close", params),
    hover: params => invoke<LanguageHoverResult>("zeta:language:hover", params),
    completions: params => invoke<LanguageCompletionsResult>("zeta:language:completions", params),
    formatDocument: params => invoke<LanguageFormattingResult>("zeta:language:formatDocument", params),
    formatRange: params => invoke<LanguageFormattingResult>("zeta:language:formatRange", params),
    signatureHelp: params => invoke<LanguageSignatureHelpResult>("zeta:language:signatureHelp", params),
    inlayHints: params => invoke<LanguageInlayHintsResult>("zeta:language:inlayHints", params),
    linkedEditingRanges: params => invoke<LanguageLinkedEditingRangesResult>("zeta:language:linkedEditingRanges", params),
    locations: params => invoke<LanguageLocationsResult>("zeta:language:locations", params),
    hierarchy: params => invoke<LanguageHierarchyResultDto>("zeta:language:hierarchy", params),
    workspaceSymbols: params => invoke<LanguageWorkspaceSymbolsResult>("zeta:language:workspaceSymbols", params),
    prepareRename: params => invoke<LanguagePrepareRenameResult>("zeta:language:prepareRename", params),
    rename: params => invoke<LanguageWorkspaceEditDto>("zeta:language:rename", params),
    codeActions: params => invoke<LanguageCodeActionsResult>("zeta:language:codeActions", params),
    resolveCodeAction: params => invoke<LanguageCodeActionDto>("zeta:language:resolveCodeAction", params),
  };
}
