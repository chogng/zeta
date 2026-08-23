import { type SyntaxWorkerFactory } from "../../common/languages/syntax/syntaxService.js";
import { SyntaxModuleWorkerClient } from "../../common/languages/syntax/syntaxModuleWorkerClient.js";
import { BrowserLanguageWorkerPort } from "./browserLanguageWorkerPort.js";

/** Creates one shared token/diagnostic module Worker for the syntax service. */
export function createSyntaxWorkerFactory(): SyntaxWorkerFactory {
	return () => new SyntaxModuleWorkerClient(
		new BrowserLanguageWorkerPort(new Worker(
			new URL("./syntaxWorkerMain.ts", import.meta.url),
			{ type: "module", name: "zeta-syntax" },
		)),
		{ requiredProviderModules: ["language.lexical"] },
	);
}
