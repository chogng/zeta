import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { LanguageAnalysisProviderModuleHost, LanguageAnalysisProviderModuleRegistry } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisProviderModules.js";
import { LanguageAnalysisProviderModuleWireServer } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisProviderModuleWire.js";
import { LanguageAnalysisProviderRegistry } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisProviders.js";
import { LanguageAnalysisProviderWorker } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisService.js";
import { languageAnalysisWireCodec } from "../../../../editor/alpha/common/languages/analysis/languageAnalysisWire.js";
import { registerBuiltinLanguageConfigurations } from "../../../../editor/alpha/common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry } from "../../../../editor/alpha/common/languages/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../../../../editor/alpha/common/languages/languageLexicalAnalysisProvider.js";
import { LanguageWorkerWireServer } from "../../../../editor/alpha/common/languages/languageWorkerWire.js";
import { createDedicatedWorkerLanguagePort } from "../../../../editor/alpha/browser/language/dedicatedWorkerLanguagePort.js";
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
resources.add(registerBuiltinLanguageConfigurations(languageConfigurations));
resources.add(modules.register({
  id: "language.lexical",
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
