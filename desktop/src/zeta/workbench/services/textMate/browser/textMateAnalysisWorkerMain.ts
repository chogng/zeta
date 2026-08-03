import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../../../editor/alpha/language/common/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../../../editor/alpha/language/common/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry } from "../../../../editor/alpha/language/common/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker } from "../../../../editor/alpha/language/common/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../../../editor/alpha/language/common/languageAnalysisWire.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../../../../editor/alpha/language/common/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../../../editor/alpha/language/common/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../../../../editor/alpha/language/common/languageLexicalAnalysisProvider.js";
import { LanguageWorkerWireServer } from "../../../../editor/alpha/language/common/languageWorkerWire.js";
import { createDedicatedWorkerLanguagePort } from "../../../../editor/alpha/language/browser/dedicatedWorkerLanguagePort.js";
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
