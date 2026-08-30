import { startZetaWorker } from '../../zetaWorkerBootstrap.js';
import { SyntaxProviderModuleHost, SyntaxProviderModuleRegistry } from '../languages/syntax/syntaxProviderModules.js';
import { SyntaxProviderModuleWireServer } from '../languages/syntax/syntaxProviderModuleWire.js';
import { SyntaxProviderRegistry } from '../languages/syntax/syntaxProviders.js';
import { SyntaxProviderWorker } from '../languages/syntax/syntaxService.js';
import { syntaxWireCodec } from '../languages/syntax/syntaxWire.js';
import { registerBuiltinLanguageConfigurations } from '../languages/languageBuiltinConfigurations.js';
import { OwnedLanguageConfigurationContributions } from '../languages/ownedLanguageConfigurationContributions.js';
import { createLanguageLexicalSyntaxProvider } from '../languages/languageLexicalSyntaxProvider.js';
import { LanguageWorkerWireServer } from '../languages/languageWorkerWire.js';

startZetaWorker(({ port, resources }) => {
	const registry = resources.add(new SyntaxProviderRegistry());
	const modules = resources.add(new SyntaxProviderModuleRegistry());
	const languageConfigurations = resources.add(new OwnedLanguageConfigurationContributions());
	resources.add(registerBuiltinLanguageConfigurations(languageConfigurations));
	resources.add(modules.register({
		id: 'language.lexical',
		load: () => [createLanguageLexicalSyntaxProvider({ languageConfigurations })],
	}));
	const moduleHost = resources.add(new SyntaxProviderModuleHost(modules, registry));
	resources.add(new LanguageWorkerWireServer(
		port,
		syntaxWireCodec,
		new SyntaxProviderWorker(registry),
	));
	resources.add(new SyntaxProviderModuleWireServer(port, modules, moduleHost));
});
