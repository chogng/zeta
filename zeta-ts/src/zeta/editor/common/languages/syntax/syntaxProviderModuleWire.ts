import { type SyntaxProvider } from "./syntaxProviders.js";
import { SyntaxProviderModuleHost, SyntaxProviderModuleRegistry } from "./syntaxProviderModules.js";
import { LanguageProviderModuleWireClient, LanguageProviderModuleWireServer, type LanguageProviderModuleWireDescriptor } from "../languageProviderModuleWire.js";
import { type LanguageWorkerWirePort } from "../languageWorkerWire.js";

const ANALYSIS_MODULE_WIRE: LanguageProviderModuleWireDescriptor = Object.freeze({
	protocol: "zeta.syntax.provider-modules",
	version: 1,
});

export class SyntaxProviderModuleWireClient extends LanguageProviderModuleWireClient {
	constructor(port: LanguageWorkerWirePort, invalidateWorker: (error: Error) => void) {
		super(port, ANALYSIS_MODULE_WIRE, invalidateWorker);
	}
}

export class SyntaxProviderModuleWireServer extends LanguageProviderModuleWireServer<SyntaxProvider> {
	constructor(port: LanguageWorkerWirePort, modules: SyntaxProviderModuleRegistry, host: SyntaxProviderModuleHost) {
		super(port, ANALYSIS_MODULE_WIRE, modules, host);
	}
}
