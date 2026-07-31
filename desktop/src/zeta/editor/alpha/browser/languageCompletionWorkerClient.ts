import { LanguageCompletionCatalogWorkerClient } from "../common/languageCompletionCatalogWire.js";
import { type LanguageCompletionWorkerFactory } from "../common/languageCompletionService.js";
import { BrowserLanguageWorkerPort } from "./browserLanguageWorkerPort.js";

/** Creates a fresh module Worker client whenever the coordinator needs a host. */
export function createAlphaCompletionWorkerFactory(): LanguageCompletionWorkerFactory {
  return () => new LanguageCompletionCatalogWorkerClient(
    new BrowserLanguageWorkerPort(new Worker(
      new URL("./languageCompletionWorkerMain.ts", import.meta.url),
      { type: "module", name: "zeta-alpha-completion" },
    )),
    { requiredProviderModules: ["alpha.word"] },
  );
}
