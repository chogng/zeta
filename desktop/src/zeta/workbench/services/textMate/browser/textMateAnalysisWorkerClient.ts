import { type LanguageAnalysisWorkerFactory } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisService.js";
import { BrowserLanguageWorkerPort } from "../../../../editor/alpha/browser/language/browserLanguageWorkerPort.js";
import { TextMateAnalysisModuleWorkerClient } from "../common/textMateAnalysisModuleWorkerClient.js";
import { type TextMateGrammarCatalogSource } from "../common/textMateGrammarCatalog.js";
import { type TextMateScopeThemeSource } from "../common/textMateScopeTheme.js";

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
      requiredProviderModules: ["textmate.grammars", "language.lexical"],
      ...(scopeTheme === undefined ? {} : { scopeTheme }),
    },
  );
}

function isThemeSource(value: unknown): value is TextMateScopeThemeSource {
  return typeof value === "object" && value !== null && "currentTheme" in value && typeof (value as TextMateScopeThemeSource).onDidChangeTheme === "function";
}

