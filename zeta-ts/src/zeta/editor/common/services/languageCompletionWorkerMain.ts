import { startZetaWorker } from '../../zetaWorkerBootstrap.js';
import { LanguageCompletionCatalogWirePublisher } from '../languages/completion/languageCompletionCatalogWire.js';
import { LanguageCompletionProviderRegistry } from '../languages/completion/languageCompletionProviders.js';
import { LanguageCompletionProviderModuleHost, LanguageCompletionProviderModuleRegistry } from '../languages/completion/languageCompletionProviderModules.js';
import { LanguageCompletionProviderModuleWireServer } from '../languages/completion/languageCompletionProviderModuleWire.js';
import { LanguageCompletionResolveWireServer } from '../languages/completion/languageCompletionResolveWire.js';
import { LanguageCompletionProviderWorker } from '../languages/completion/languageCompletionService.js';
import { languageCompletionWireCodec } from '../languages/completion/languageCompletionWire.js';
import { createLanguageWordCompletionProvider } from '../languages/completion/languageWordCompletionProvider.js';
import { LanguageWorkerWireServer } from '../languages/languageWorkerWire.js';

startZetaWorker(({ port, resources }) => {
	const registry = resources.add(new LanguageCompletionProviderRegistry());
	const modules = resources.add(new LanguageCompletionProviderModuleRegistry());
	resources.add(modules.register({
		id: 'language.word',
		load: () => [createLanguageWordCompletionProvider()],
	}));
	const moduleHost = resources.add(new LanguageCompletionProviderModuleHost(modules, registry));
	const providerWorker = new LanguageCompletionProviderWorker(registry);
	resources.add(new LanguageWorkerWireServer(
		port,
		languageCompletionWireCodec,
		providerWorker,
	));
	resources.add(new LanguageCompletionCatalogWirePublisher(port, registry));
	resources.add(new LanguageCompletionProviderModuleWireServer(port, modules, moduleHost));
	resources.add(new LanguageCompletionResolveWireServer(port, providerWorker));
});
