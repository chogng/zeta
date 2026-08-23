import type { LanguageCodeActionDto, LanguageCodeActionsResult, LanguageCodeLensesResult, LanguageColorPresentationsResult, LanguageCompletionDetailsResult, LanguageCompletionsResult, LanguageDocumentColorsResult, LanguageDocumentDiagnosticsResult, LanguageDocumentLinksResult, LanguageDocumentSymbolsResult, LanguageFoldingRangesResult, LanguageFormattingResult, LanguageHierarchyResultDto, LanguageHoverResult, LanguageInlayHintsResult, LanguageLinkedEditingRangesResult, LanguageLocationsResult, LanguagePrepareRenameResult, LanguageSemanticTokensResult, LanguageSignatureHelpResult, LanguageWorkspaceDiagnosticsResult, LanguageWorkspaceEditDto, LanguageWorkspaceSymbolsResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { ILanguageApi } from "../common/languageApi.js";

export function createLanguageApi(): ILanguageApi {
  return {
    synchronize: params => invoke<void>("zeta:language:synchronize", params),
    close: params => invoke<void>("zeta:language:close", params),
    hover: params => invoke<LanguageHoverResult>("zeta:language:hover", params),
    completions: params => invoke<LanguageCompletionsResult>("zeta:language:completions", params),
    resolveCompletion: params => invoke<LanguageCompletionDetailsResult>("zeta:language:resolveCompletion", params),
    executeCommand: params => invoke<void>("zeta:language:executeCommand", params),
    documentDiagnostics: params => invoke<LanguageDocumentDiagnosticsResult>("zeta:language:documentDiagnostics", params),
    workspaceDiagnostics: params => invoke<LanguageWorkspaceDiagnosticsResult>("zeta:language:workspaceDiagnostics", params),
    formatDocument: params => invoke<LanguageFormattingResult>("zeta:language:formatDocument", params),
    formatRange: params => invoke<LanguageFormattingResult>("zeta:language:formatRange", params),
    signatureHelp: params => invoke<LanguageSignatureHelpResult>("zeta:language:signatureHelp", params),
    inlayHints: params => invoke<LanguageInlayHintsResult>("zeta:language:inlayHints", params),
    linkedEditingRanges: params => invoke<LanguageLinkedEditingRangesResult>("zeta:language:linkedEditingRanges", params),
    semanticTokens: params => invoke<LanguageSemanticTokensResult>("zeta:language:semanticTokens", params),
    documentSymbols: params => invoke<LanguageDocumentSymbolsResult>("zeta:language:documentSymbols", params),
    codeLenses: params => invoke<LanguageCodeLensesResult>("zeta:language:codeLenses", params),
    resolveCodeLens: params => invoke<LanguageCodeLensesResult>("zeta:language:resolveCodeLens", params),
    documentLinks: params => invoke<LanguageDocumentLinksResult>("zeta:language:documentLinks", params),
    resolveDocumentLink: params => invoke<LanguageDocumentLinksResult>("zeta:language:resolveDocumentLink", params),
    documentColors: params => invoke<LanguageDocumentColorsResult>("zeta:language:documentColors", params),
    colorPresentations: params => invoke<LanguageColorPresentationsResult>("zeta:language:colorPresentations", params),
    foldingRanges: params => invoke<LanguageFoldingRangesResult>("zeta:language:foldingRanges", params),
    locations: params => invoke<LanguageLocationsResult>("zeta:language:locations", params),
    hierarchy: params => invoke<LanguageHierarchyResultDto>("zeta:language:hierarchy", params),
    workspaceSymbols: params => invoke<LanguageWorkspaceSymbolsResult>("zeta:language:workspaceSymbols", params),
    prepareRename: params => invoke<LanguagePrepareRenameResult>("zeta:language:prepareRename", params),
    rename: params => invoke<LanguageWorkspaceEditDto>("zeta:language:rename", params),
    codeActions: params => invoke<LanguageCodeActionsResult>("zeta:language:codeActions", params),
    resolveCodeAction: params => invoke<LanguageCodeActionDto>("zeta:language:resolveCodeAction", params),
  };
}
