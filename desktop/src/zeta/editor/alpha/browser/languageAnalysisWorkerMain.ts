import { DisposableStore } from "../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../common/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../common/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry } from "../common/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker } from "../common/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../common/languageAnalysisWire.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../common/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../common/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../common/languageLexicalAnalysisProvider.js";
import { LanguageWorkerWireServer } from "../common/languageWorkerWire.js";
import { createDedicatedWorkerLanguagePort } from "./dedicatedWorkerLanguagePort.js";

const resources = new DisposableStore();
const registry = resources.add(new LanguageAnalysisProviderRegistry());
const modules = resources.add(new LanguageAnalysisProviderModuleRegistry());
const languageConfigurations = resources.add(new LanguageConfigurationRegistry());
resources.add(registerAlphaBuiltinLanguageConfigurations(languageConfigurations));
resources.add(modules.register({
  id: "alpha.lexical",
  load: () => [createLanguageLexicalAnalysisProvider({ languageConfigurations })],
}));
const moduleHost = resources.add(new LanguageAnalysisProviderModuleHost(modules, registry));
const port = createDedicatedWorkerLanguagePort();
resources.add(new LanguageWorkerWireServer(
  port,
  languageAnalysisWireCodec,
  new LanguageAnalysisProviderWorker(registry),
));
resources.add(new LanguageAnalysisProviderModuleWireServer(port, modules, moduleHost));
