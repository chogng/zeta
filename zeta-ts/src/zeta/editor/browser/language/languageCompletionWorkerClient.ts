import { LanguageCompletionCatalogWorkerClient } from "../../common/languages/completion/languageCompletionCatalogWire.js";
import { type LanguageCompletionWorkerFactory } from "../../common/languages/completion/languageCompletionService.js";
import { BrowserLanguageWorkerPort } from "./browserLanguageWorkerPort.js";

/** Creates a fresh module Worker client whenever the coordinator needs a host. */
export function createCompletionWorkerFactory(): LanguageCompletionWorkerFactory {
	return () => new LanguageCompletionCatalogWorkerClient(
		new BrowserLanguageWorkerPort(new Worker(
			new URL("./languageCompletionWorkerMain.ts", import.meta.url),
			{ type: "module", name: "stanza-completion" },
		)),
		{ requiredProviderModules: ["language.word"] },
	);
}
