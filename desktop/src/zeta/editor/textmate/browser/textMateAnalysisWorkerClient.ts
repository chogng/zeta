import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type LanguageAnalysisWorkerFactory } from "../../alpha/common/languageAnalysisService.js";
import { BrowserLanguageWorkerPort } from "../../alpha/browser/browserLanguageWorkerPort.js";
import { TextMateAnalysisModuleWorkerClient } from "../common/textMateAnalysisModuleWorkerClient.js";
import { type TextMateGrammarCatalogSource } from "../common/textMateGrammarCatalog.js";
import { BrowserTextMateGrammarService } from "./browserTextMateGrammarService.js";

/** Owns the built-in grammar catalog and matching dedicated Analysis Worker factory. */
export class BrowserTextMateAnalysisWorkerSupport extends DisposableOwner {
  readonly grammars = this.own(new BrowserTextMateGrammarService());
  readonly workerFactory = createTextMateAnalysisWorkerFactory(this.grammars);
}

/** Creates an Analysis Worker gated by a renderer-owned TextMate grammar catalog. */
export function createTextMateAnalysisWorkerFactory(catalogs: TextMateGrammarCatalogSource): LanguageAnalysisWorkerFactory {
  if (!catalogs || typeof catalogs !== "object" || typeof catalogs.onDidChangeCatalog !== "function" || !("currentCatalog" in catalogs)) {
    throw new TypeError("TextMate Analysis Worker factory requires a grammar catalog source");
  }
  return () => new TextMateAnalysisModuleWorkerClient(
    new BrowserLanguageWorkerPort(new Worker(
      new URL("./textMateAnalysisWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-textmate-analysis" },
    )),
    catalogs,
    { requiredProviderModules: ["textmate.grammars", "alpha.lexical"] },
  );
}
