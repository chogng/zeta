import { LanguageLexicalSyntaxCache, type LanguageLexicalCacheUpdateObserver } from "./languageLexicalSyntaxCache.js";
import { type SyntaxProvider, type SyntaxProviderRequest } from "./syntax/syntaxProviders.js";
import { BUILTIN_LANGUAGE_IDS, createBuiltinLanguageConfigurationService } from './languageBuiltinConfigurations.js';
import { type ILanguageConfigurationService, type ResolvedLanguageConfiguration } from './languageConfigurationRegistry.js';
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageWorkerDocumentSynchronization } from '../services/textModelSync/textModelSync.protocol.js';

export interface LanguageLexicalSyntaxProviderOptions {
	readonly onDidUpdateCache?: LanguageLexicalCacheUpdateObserver;
	readonly languageConfigurations?: ILanguageConfigurationService;
}

interface LanguageCacheEntry {
	readonly configuration: ResolvedLanguageConfiguration;
	readonly cache: LanguageLexicalSyntaxCache;
}

/** Creates the incremental deterministic baseline tokenizer and structural diagnostics. */
export function createLanguageLexicalSyntaxProvider(options: LanguageLexicalSyntaxProviderOptions = {}): SyntaxProvider {
	if (typeof options !== "object" || options === null) {
		throw new TypeError("Language lexical syntax provider options must be an object");
	}
	const languageConfigurations = options.languageConfigurations ?? createBuiltinLanguageConfigurationService();
	if (!languageConfigurations || typeof languageConfigurations.getLanguageConfiguration !== "function") {
		throw new TypeError("Language lexical syntax provider requires a language configuration source");
	}
	const caches = new Map<string, LanguageCacheEntry>();
	const getCache = (languageId: string): LanguageLexicalSyntaxCache => {
		const configuration = languageConfigurations.getLanguageConfiguration(languageId);
		const current = caches.get(languageId);
		if (current?.configuration === configuration) return current.cache;
		const cache = new LanguageLexicalSyntaxCache({
			scanner: createLanguageLexicalLineScanner(languageId, configuration),
			onDidUpdate: options.onDidUpdateCache,
		});
		caches.set(languageId, { configuration, cache });
		return cache;
	};
	return Object.freeze({
		id: "language.lexical",
		languageIds: BUILTIN_LANGUAGE_IDS,
		provideTokens: (request: SyntaxProviderRequest, signal: AbortSignal) => getCache(request.languageId).getTokens(request.snapshot, signal),
		provideDiagnostics: (request: SyntaxProviderRequest, signal: AbortSignal) => getCache(request.languageId).getDiagnostics(request.snapshot, signal),
		synchronizeDocument: (synchronization: LanguageWorkerDocumentSynchronization) => {
			for (const entry of caches.values()) entry.cache.synchronizeDocument(synchronization);
		},
	});
}
