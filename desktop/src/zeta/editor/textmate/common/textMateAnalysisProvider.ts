import { type LanguageAnalysisProvider, type LanguageAnalysisProviderRequest } from "../../alpha/common/languageAnalysisProviders.js";
import { type LanguageWorkerDocumentSynchronization } from "../../alpha/common/languageWorkerDocumentMirror.js";
import { TextMateTokenizationService } from "./textMateTokenizationService.js";

export const TEXTMATE_ANALYSIS_PROVIDER_ID = "textmate.grammar";
export const TEXTMATE_TOKEN_PRIORITY = 100;

/** Adapts one caller-owned TextMate runtime to Alpha's Analysis provider contract. */
export function createTextMateAnalysisProvider(tokenization: TextMateTokenizationService): LanguageAnalysisProvider {
  if (!(tokenization instanceof TextMateTokenizationService)) {
    throw new TypeError("TextMate analysis provider requires a tokenization service");
  }
  return Object.freeze({
    id: TEXTMATE_ANALYSIS_PROVIDER_ID,
    languageIds: Object.freeze(["*"]),
    tokenPriority: TEXTMATE_TOKEN_PRIORITY,
    provideTokens: (request: LanguageAnalysisProviderRequest, signal: AbortSignal) => tokenization.tokenize(request.languageId, request.snapshot, signal),
    synchronizeDocument: (synchronization: LanguageWorkerDocumentSynchronization) => tokenization.synchronizeDocument(synchronization),
  });
}
