import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LanguageAnalysisWorkerFactory } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisService.js";
import { type ITextMateService } from "../common/textMateService.js";
import { type TextMateGrammarDefinition } from "../common/textMateGrammarRegistry.js";
import { TextMateScopeThemeModel, type TextMateScopeThemeSource } from "../common/textMateScopeTheme.js";
import { BrowserTextMateGrammarService } from "./browserTextMateGrammarService.js";
import { createTextMateAnalysisWorkerFactory } from "./textMateAnalysisWorkerClient.js";

/** Browser implementation of the Workbench TextMate service. */
export class BrowserTextMateService extends DisposableOwner implements ITextMateService {
  readonly grammars = this.own(new BrowserTextMateGrammarService());
  readonly scopeTheme: TextMateScopeThemeSource;
  readonly analysisWorkerFactory: LanguageAnalysisWorkerFactory;

  constructor(contributions: readonly TextMateGrammarDefinition[] = [], scopeTheme?: TextMateScopeThemeSource) {
    super();
    if (!Array.isArray(contributions)) {
      this.dispose();
      throw new TypeError("Browser TextMate grammar contributions must be an array");
    }
    try {
      if (scopeTheme !== undefined && !isThemeSource(scopeTheme)) {
        throw new TypeError("Browser TextMate scope theme must be a theme source");
      }
      this.scopeTheme = scopeTheme ?? this.own(new TextMateScopeThemeModel());
      this.analysisWorkerFactory = createTextMateAnalysisWorkerFactory(this.grammars, this.scopeTheme);
      for (const contribution of contributions) this.grammars.registerGrammar(contribution);
    } catch (error) {
      this.dispose();
      throw error;
    }
  }
}

function isThemeSource(value: unknown): value is TextMateScopeThemeSource {
  return typeof value === "object" && value !== null && "currentTheme" in value && typeof (value as TextMateScopeThemeSource).onDidChangeTheme === "function";
}
