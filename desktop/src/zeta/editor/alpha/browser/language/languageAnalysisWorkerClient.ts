import { type LanguageAnalysisWorkerFactory } from "../../common/languages/analysis/languageAnalysisService.js";
import { LanguageAnalysisModuleWorkerClient } from "../../common/languages/analysis/languageAnalysisModuleWorkerClient.js";
import { BrowserLanguageWorkerPort } from "./browserLanguageWorkerPort.js";

/** Creates one shared token/diagnostic module Worker for an analysis service. */
export function createAnalysisWorkerFactory(): LanguageAnalysisWorkerFactory {
  return () => new LanguageAnalysisModuleWorkerClient(
    new BrowserLanguageWorkerPort(new Worker(
      new URL("./languageAnalysisWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-analysis" },
    )),
    { requiredProviderModules: ["language.lexical"] },
  );
}
