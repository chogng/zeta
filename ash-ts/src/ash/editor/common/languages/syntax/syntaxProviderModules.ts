import { type SyntaxProvider, SyntaxProviderRegistry } from "./syntaxProviders.js";
import { LanguageProviderModuleHost, LanguageProviderModuleRegistry, LanguageProviderModuleState as SyntaxProviderModuleState, normalizeLanguageProviderModuleCatalog, type LanguageProviderModule, type LanguageProviderModuleCatalog, type LanguageProviderModuleCatalogSource, type LanguageProviderModuleController, type LanguageProviderModuleMetadata, type LanguageProviderModuleStateChange } from "../languageProviderModules.js";

export { SyntaxProviderModuleState };

export type SyntaxProviderModule = LanguageProviderModule<SyntaxProvider>;
export type SyntaxProviderModuleMetadata = LanguageProviderModuleMetadata;
export type SyntaxProviderModuleCatalog = LanguageProviderModuleCatalog;
export type SyntaxProviderModuleCatalogSource = LanguageProviderModuleCatalogSource;
export type SyntaxProviderModuleController = LanguageProviderModuleController;
export type SyntaxProviderModuleStateChange = LanguageProviderModuleStateChange;

export class SyntaxProviderModuleRegistry extends LanguageProviderModuleRegistry<SyntaxProvider> {}

export class SyntaxProviderModuleHost extends LanguageProviderModuleHost<SyntaxProvider> {
	constructor(modules: SyntaxProviderModuleRegistry, providers: SyntaxProviderRegistry) {
		super(modules, providers);
	}
}

export function normalizeSyntaxProviderModuleCatalog(value: unknown): SyntaxProviderModuleCatalog {
	return normalizeLanguageProviderModuleCatalog(value);
}
