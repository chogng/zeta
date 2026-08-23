import { type LanguageCompletionProvider, LanguageCompletionProviderRegistry } from "./languageCompletionProviders.js";
import { LanguageProviderModuleHost, LanguageProviderModuleRegistry, LanguageProviderModuleState as LanguageCompletionProviderModuleState, normalizeLanguageProviderModuleCatalog, type LanguageProviderModule, type LanguageProviderModuleCatalog, type LanguageProviderModuleCatalogSource, type LanguageProviderModuleController, type LanguageProviderModuleMetadata, type LanguageProviderModuleStateChange } from "../languageProviderModules.js";

export { LanguageCompletionProviderModuleState };

export type LanguageCompletionProviderModule = LanguageProviderModule<LanguageCompletionProvider>;
export type LanguageCompletionProviderModuleMetadata = LanguageProviderModuleMetadata;
export type LanguageCompletionProviderModuleCatalog = LanguageProviderModuleCatalog;
export type LanguageCompletionProviderModuleCatalogSource = LanguageProviderModuleCatalogSource;
export type LanguageCompletionProviderModuleController = LanguageProviderModuleController;
export type LanguageCompletionProviderModuleStateChange = LanguageProviderModuleStateChange;

export class LanguageCompletionProviderModuleRegistry extends LanguageProviderModuleRegistry<LanguageCompletionProvider> {}

export class LanguageCompletionProviderModuleHost extends LanguageProviderModuleHost<LanguageCompletionProvider> {
	constructor(modules: LanguageCompletionProviderModuleRegistry, providers: LanguageCompletionProviderRegistry) {
		super(modules, providers);
	}
}

export function normalizeLanguageCompletionProviderModuleCatalog(value: unknown): LanguageCompletionProviderModuleCatalog {
	return normalizeLanguageProviderModuleCatalog(value);
}
