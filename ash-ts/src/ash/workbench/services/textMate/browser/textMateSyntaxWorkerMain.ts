import { start } from "../../../../editor/editor.worker.start.js";
import { SyntaxProviderModuleHost, SyntaxProviderModuleRegistry } from "../../../../editor/common/languages/syntax/syntaxProviderModules.js";
import { SyntaxProviderModuleWireServer } from "../../../../editor/common/languages/syntax/syntaxProviderModuleWire.js";
import { SyntaxProviderRegistry } from "../../../../editor/common/languages/syntax/syntaxProviders.js";
import { SyntaxProviderWorker } from "../../../../editor/common/languages/syntax/syntaxService.js";
import { syntaxWireCodec } from "../../../../editor/common/languages/syntax/syntaxWire.js";
import { registerBuiltinLanguageConfigurations } from "../../../../editor/common/languages/languageBuiltinConfigurations.js";
import { LanguageConfigurationService } from "../../../../editor/common/languages/languageConfigurationRegistry.js";
import { createLanguageLexicalSyntaxProvider } from "../../../../editor/common/languages/languageLexicalSyntaxProvider.js";
import { LanguageWorkerWireServer } from "../../../../editor/common/languages/languageWorkerWire.js";
import { createTextMateSyntaxModule } from "../common/textMateSyntaxModule.js";
import { TextMateGrammarCatalogStore } from "../common/textMateGrammarCatalogStore.js";
import { TextMateGrammarCatalogWireServer } from "../common/textMateGrammarCatalogWire.js";
import { TextMateScopeThemeModel } from "../common/textMateScopeTheme.js";
import { TextMateScopeThemeWireServer } from "../common/textMateScopeThemeWire.js";
import { createBrowserTextMateTokenizationService } from "./browserTextMateTokenization.js";
import { InMemoryConfigurationService } from "../../../../platform/configuration/common/inMemoryConfigurationService.js";
import { LanguageService } from "../../../../editor/common/services/languageService.js";

start(({ port, resources }) => {
	const registry = resources.add(new SyntaxProviderRegistry());
	const modules = resources.add(new SyntaxProviderModuleRegistry());
	const configurationService = resources.add(new InMemoryConfigurationService());
	const languageService = resources.add(new LanguageService());
	const languageConfigurations = resources.add(new LanguageConfigurationService(configurationService, languageService));
	resources.add(registerBuiltinLanguageConfigurations(languageConfigurations));
	resources.add(modules.register({
		id: "language.lexical",
		load: () => [createLanguageLexicalSyntaxProvider({ languageConfigurations })],
	}));
	const grammarCatalog = resources.add(new TextMateGrammarCatalogStore());
	const scopeTheme = resources.add(new TextMateScopeThemeModel());
	const textMateTokenization = resources.add(createBrowserTextMateTokenizationService(grammarCatalog, {
		scopeResolver: scopes => scopeTheme.resolve(scopes),
	}));
	resources.add(modules.register(createTextMateSyntaxModule(textMateTokenization)));
	const moduleHost = resources.add(new SyntaxProviderModuleHost(modules, registry));
	resources.add(new LanguageWorkerWireServer(port, syntaxWireCodec, new SyntaxProviderWorker(registry)));
	resources.add(new SyntaxProviderModuleWireServer(port, modules, moduleHost));
	resources.add(new TextMateGrammarCatalogWireServer(port, grammarCatalog));
	resources.add(new TextMateScopeThemeWireServer(port, scopeTheme, () => textMateTokenization.invalidateTokenCaches()));
});
