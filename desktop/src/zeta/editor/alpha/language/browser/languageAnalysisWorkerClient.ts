import { type LanguageAnalysisWorkerFactory } from "../common/languageAnalysisService.js";
import { LanguageAnalysisModuleWorkerClient } from "../common/languageAnalysisModuleWorkerClient.js";
import { BrowserLanguageWorkerPort } from "./browserLanguageWorkerPort.js";

/** Creates one shared token/diagnostic module Worker for an analysis service. */
export function createAlphaAnalysisWorkerFactory(): LanguageAnalysisWorkerFactory {
  return () => new LanguageAnalysisModuleWorkerClient(
    new BrowserLanguageWorkerPort(new Worker(
      new URL("./languageAnalysisWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-alpha-analysis" },
    )),
    { requiredProviderModules: ["alpha.lexical"] },
  );
}
