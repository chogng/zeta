import { LanguageLexicalAnalysisCache, type LanguageLexicalCacheUpdateObserver } from "./languageLexicalAnalysisCache.js";
import { type LanguageAnalysisProvider, type LanguageAnalysisProviderRequest } from "./languageAnalysisProviders.js";
import { ALPHA_BUILTIN_LANGUAGE_IDS, createAlphaBuiltinLanguageConfigurationSource } from "./languageBuiltinConfigurations.js";
import { type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageWorkerDocumentSynchronization } from "./languageWorkerDocumentMirror.js";

export interface LanguageLexicalAnalysisProviderOptions {
  readonly onDidUpdateCache?: LanguageLexicalCacheUpdateObserver;
  readonly languageConfigurations?: LanguageConfigurationSource;
}

interface LanguageCacheEntry {
  readonly configuration: ResolvedLanguageConfiguration;
  readonly cache: LanguageLexicalAnalysisCache;
}

/** Creates Alpha's incremental deterministic baseline tokenizer and structural diagnostics. */
export function createLanguageLexicalAnalysisProvider(options: LanguageLexicalAnalysisProviderOptions = {}): LanguageAnalysisProvider {
  if (typeof options !== "object" || options === null) {
    throw new TypeError("Language lexical analysis provider options must be an object");
  }
  const languageConfigurations = options.languageConfigurations ?? createAlphaBuiltinLanguageConfigurationSource();
  if (!languageConfigurations || typeof languageConfigurations.getLanguageConfiguration !== "function") {
    throw new TypeError("Language lexical analysis provider requires a language configuration source");
  }
  const caches = new Map<string, LanguageCacheEntry>();
  const getCache = (languageId: string): LanguageLexicalAnalysisCache => {
    const configuration = languageConfigurations.getLanguageConfiguration(languageId);
    const current = caches.get(languageId);
    if (current?.configuration === configuration) return current.cache;
    const cache = new LanguageLexicalAnalysisCache({
      scanner: createLanguageLexicalLineScanner(languageId, configuration),
      onDidUpdate: options.onDidUpdateCache,
    });
    caches.set(languageId, { configuration, cache });
    return cache;
  };
  return Object.freeze({
    id: "alpha.lexical",
    languageIds: ALPHA_BUILTIN_LANGUAGE_IDS,
    provideTokens: (request: LanguageAnalysisProviderRequest, signal: AbortSignal) => getCache(request.languageId).getTokens(request.snapshot, signal),
    provideDiagnostics: (request: LanguageAnalysisProviderRequest, signal: AbortSignal) => getCache(request.languageId).getDiagnostics(request.snapshot, signal),
    synchronizeDocument: (synchronization: LanguageWorkerDocumentSynchronization) => {
      for (const entry of caches.values()) entry.cache.synchronizeDocument(synchronization);
    },
  });
}
