import { type LanguageCompletionProvider } from "./languageCompletionProviders.js";
import { LanguageCompletionProviderModuleHost, LanguageCompletionProviderModuleRegistry } from "./languageCompletionProviderModules.js";
import { LanguageProviderModuleWireClient, LanguageProviderModuleWireServer, type LanguageProviderModuleWireDescriptor } from "../languageProviderModuleWire.js";
import { type LanguageWorkerWirePort } from "../languageWorkerWire.js";

const COMPLETION_MODULE_WIRE: LanguageProviderModuleWireDescriptor = Object.freeze({
  protocol: "zeta.language.completion-provider-modules",
  version: 1,
});

export class LanguageCompletionProviderModuleWireClient extends LanguageProviderModuleWireClient {
  constructor(port: LanguageWorkerWirePort, invalidateWorker: (error: Error) => void) {
    super(port, COMPLETION_MODULE_WIRE, invalidateWorker);
  }
}

export class LanguageCompletionProviderModuleWireServer extends LanguageProviderModuleWireServer<LanguageCompletionProvider> {
  constructor(port: LanguageWorkerWirePort, modules: LanguageCompletionProviderModuleRegistry, host: LanguageCompletionProviderModuleHost) {
    super(port, COMPLETION_MODULE_WIRE, modules, host);
  }
}
