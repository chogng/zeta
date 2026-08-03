import { DisposableStore } from "../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../alpha/common/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../alpha/common/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry } from "../../alpha/common/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker } from "../../alpha/common/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../alpha/common/languageAnalysisWire.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../../alpha/common/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../alpha/common/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../../alpha/common/languageLexicalAnalysisProvider.js";
import { LanguageWorkerWireServer } from "../../alpha/common/languageWorkerWire.js";
import { createDedicatedWorkerLanguagePort } from "../../alpha/browser/dedicatedWorkerLanguagePort.js";
import { createTextMateAnalysisModule } from "../common/textMateAnalysisModule.js";
import { TextMateGrammarCatalogStore } from "../common/textMateGrammarCatalogStore.js";
import { TextMateGrammarCatalogWireServer } from "../common/textMateGrammarCatalogWire.js";
import { TextMateScopeThemeModel } from "../common/textMateScopeTheme.js";
import { TextMateScopeThemeWireServer } from "../common/textMateScopeThemeWire.js";
import { createBrowserTextMateTokenizationService } from "./browserTextMateTokenization.js";

const resources = new DisposableStore();
const registry = resources.add(new LanguageAnalysisProviderRegistry());
const modules = resources.add(new LanguageAnalysisProviderModuleRegistry());
const languageConfigurations = resources.add(new LanguageConfigurationRegistry());
resources.add(registerAlphaBuiltinLanguageConfigurations(languageConfigurations));
resources.add(modules.register({
  id: "alpha.lexical",
  load: () => [createLanguageLexicalAnalysisProvider({ languageConfigurations })],
}));
const grammarCatalog = resources.add(new TextMateGrammarCatalogStore());
const scopeTheme = resources.add(new TextMateScopeThemeModel());
const textMateTokenization = resources.add(createBrowserTextMateTokenizationService(grammarCatalog, {
  scopeResolver: scopes => scopeTheme.resolve(scopes),
}));
resources.add(modules.register(createTextMateAnalysisModule(textMateTokenization)));
const moduleHost = resources.add(new LanguageAnalysisProviderModuleHost(modules, registry));
const port = createDedicatedWorkerLanguagePort();
resources.add(new LanguageWorkerWireServer(port, languageAnalysisWireCodec, new LanguageAnalysisProviderWorker(registry)));
resources.add(new LanguageAnalysisProviderModuleWireServer(port, modules, moduleHost));
resources.add(new TextMateGrammarCatalogWireServer(port, grammarCatalog));
resources.add(new TextMateScopeThemeWireServer(port, scopeTheme, () => textMateTokenization.invalidateTokenCaches()));
