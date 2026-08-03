import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type LanguageAnalysisWorkerFactory } from "../../alpha/common/languageAnalysisService.js";
import { BrowserLanguageWorkerPort } from "../../alpha/browser/browserLanguageWorkerPort.js";
import { TextMateAnalysisModuleWorkerClient } from "../common/textMateAnalysisModuleWorkerClient.js";
import { type TextMateGrammarCatalogSource } from "../common/textMateGrammarCatalog.js";
import { BrowserTextMateGrammarService } from "./browserTextMateGrammarService.js";
import { type TextMateGrammarDefinition } from "../common/textMateGrammarRegistry.js";
import { TextMateScopeThemeModel, type TextMateScopeThemeSource } from "../common/textMateScopeTheme.js";

/** Owns the built-in grammar catalog and matching dedicated Analysis Worker factory. */
export class BrowserTextMateAnalysisWorkerSupport extends DisposableOwner {
  readonly grammars = this.own(new BrowserTextMateGrammarService());
  readonly scopeTheme: TextMateScopeThemeSource;
  readonly workerFactory: LanguageAnalysisWorkerFactory;

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
      this.workerFactory = createTextMateAnalysisWorkerFactory(this.grammars, this.scopeTheme);
      for (const contribution of contributions) this.grammars.registerGrammar(contribution);
    } catch (error) {
      this.dispose();
      throw error;
    }
  }
}

/** Creates an Analysis Worker gated by a renderer-owned TextMate grammar catalog. */
export function createTextMateAnalysisWorkerFactory(catalogs: TextMateGrammarCatalogSource, scopeTheme?: TextMateScopeThemeSource): LanguageAnalysisWorkerFactory {
  if (!catalogs || typeof catalogs !== "object" || typeof catalogs.onDidChangeCatalog !== "function" || !("currentCatalog" in catalogs)) {
    throw new TypeError("TextMate Analysis Worker factory requires a grammar catalog source");
  }
  if (scopeTheme !== undefined && !isThemeSource(scopeTheme)) {
    throw new TypeError("TextMate Analysis Worker factory scope theme must be a theme source");
  }
  return () => new TextMateAnalysisModuleWorkerClient(
    new BrowserLanguageWorkerPort(new Worker(
      new URL("./textMateAnalysisWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-textmate-analysis" },
    )),
    catalogs,
    {
      requiredProviderModules: ["textmate.grammars", "alpha.lexical"],
      ...(scopeTheme === undefined ? {} : { scopeTheme }),
    },
  );
}

function isThemeSource(value: unknown): value is TextMateScopeThemeSource {
  return typeof value === "object" && value !== null && "currentTheme" in value && typeof (value as TextMateScopeThemeSource).onDidChangeTheme === "function";
}
