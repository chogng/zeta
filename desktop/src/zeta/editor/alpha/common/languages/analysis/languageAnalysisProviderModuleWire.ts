import { type LanguageAnalysisProvider } from "./languageAnalysisProviders.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "./languageAnalysisProviderModules.js";
import { LanguageProviderModuleWireClient, LanguageProviderModuleWireServer, type LanguageProviderModuleWireDescriptor } from "../languageProviderModuleWire.js";
import { type LanguageWorkerWirePort } from "../languageWorkerWire.js";

const ANALYSIS_MODULE_WIRE: LanguageProviderModuleWireDescriptor = Object.freeze({
  protocol: "zeta.language.analysis-provider-modules",
  version: 1,
});

export class LanguageAnalysisProviderModuleWireClient extends LanguageProviderModuleWireClient {
  constructor(port: LanguageWorkerWirePort, invalidateWorker: (error: Error) => void) {
    super(port, ANALYSIS_MODULE_WIRE, invalidateWorker);
  }
}

export class LanguageAnalysisProviderModuleWireServer extends LanguageProviderModuleWireServer<LanguageAnalysisProvider> {
  constructor(port: LanguageWorkerWirePort, modules: LanguageAnalysisProviderModuleRegistry, host: LanguageAnalysisProviderModuleHost) {
    super(port, ANALYSIS_MODULE_WIRE, modules, host);
  }
}
