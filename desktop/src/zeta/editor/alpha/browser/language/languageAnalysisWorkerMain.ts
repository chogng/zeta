import { start } from "../../editor.worker.start.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../common/languages/analysis/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../common/languages/analysis/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry } from "../../common/languages/analysis/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker } from "../../common/languages/analysis/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../common/languages/analysis/languageAnalysisWire.js";
import { registerBuiltinLanguageConfigurations } from "../../common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../common/languages/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../../common/languages/languageLexicalAnalysisProvider.js";
import { LanguageWorkerWireServer } from "../../common/languages/languageWorkerWire.js";

start(({ port, resources }) => {
  const registry = resources.add(new LanguageAnalysisProviderRegistry());
  const modules = resources.add(new LanguageAnalysisProviderModuleRegistry());
  const languageConfigurations = resources.add(new LanguageConfigurationRegistry());
  resources.add(registerBuiltinLanguageConfigurations(languageConfigurations));
  resources.add(modules.register({
    id: "language.lexical",
    load: () => [createLanguageLexicalAnalysisProvider({ languageConfigurations })],
  }));
  const moduleHost = resources.add(new LanguageAnalysisProviderModuleHost(modules, registry));
  resources.add(new LanguageWorkerWireServer(
    port,
    languageAnalysisWireCodec,
    new LanguageAnalysisProviderWorker(registry),
  ));
  resources.add(new LanguageAnalysisProviderModuleWireServer(port, modules, moduleHost));
});
