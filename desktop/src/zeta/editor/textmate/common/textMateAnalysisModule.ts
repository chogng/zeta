import { type LanguageAnalysisProviderModule } from "../../alpha/common/languageAnalysisProviderModules.js";
import { createTextMateAnalysisProvider } from "./textMateAnalysisProvider.js";
import { TextMateTokenizationService } from "./textMateTokenizationService.js";

export const TEXTMATE_ANALYSIS_MODULE_ID = "textmate.grammars";

/** Creates a provider module without transferring ownership of its TextMate service. */
export function createTextMateAnalysisModule(tokenization: TextMateTokenizationService): LanguageAnalysisProviderModule {
  if (!(tokenization instanceof TextMateTokenizationService)) {
    throw new TypeError("TextMate analysis module requires a tokenization service");
  }
  return Object.freeze({
    id: TEXTMATE_ANALYSIS_MODULE_ID,
    load: () => Object.freeze([createTextMateAnalysisProvider(tokenization)]),
  });
}
