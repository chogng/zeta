import { type SyntaxProviderModule } from "../../../../editor/common/languages/syntax/syntaxProviderModules.js";
import { createTextMateSyntaxProvider } from "./textMateSyntaxProvider.js";
import { TextMateTokenizationService } from "./textMateTokenizationService.js";

export const TEXTMATE_SYNTAX_MODULE_ID = "textmate.grammars";

/** Creates a provider module without transferring ownership of its TextMate service. */
export function createTextMateSyntaxModule(tokenization: TextMateTokenizationService): SyntaxProviderModule {
  if (!(tokenization instanceof TextMateTokenizationService)) {
    throw new TypeError("TextMate syntax module requires a tokenization service");
  }
  return Object.freeze({
    id: TEXTMATE_SYNTAX_MODULE_ID,
    load: () => Object.freeze([createTextMateSyntaxProvider(tokenization)]),
  });
}
