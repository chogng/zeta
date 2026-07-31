import { type LanguageAnalysisProvider, LanguageAnalysisProviderRegistry } from "./languageAnalysisProviders.js";
import { LanguageProviderModuleHost, LanguageProviderModuleRegistry, LanguageProviderModuleState as LanguageAnalysisProviderModuleState, normalizeLanguageProviderModuleCatalog, type LanguageProviderModule, type LanguageProviderModuleCatalog, type LanguageProviderModuleCatalogSource, type LanguageProviderModuleController, type LanguageProviderModuleMetadata, type LanguageProviderModuleStateChange } from "./languageProviderModules.js";

export { LanguageAnalysisProviderModuleState };

export type LanguageAnalysisProviderModule = LanguageProviderModule<LanguageAnalysisProvider>;
export type LanguageAnalysisProviderModuleMetadata = LanguageProviderModuleMetadata;
export type LanguageAnalysisProviderModuleCatalog = LanguageProviderModuleCatalog;
export type LanguageAnalysisProviderModuleCatalogSource = LanguageProviderModuleCatalogSource;
export type LanguageAnalysisProviderModuleController = LanguageProviderModuleController;
export type LanguageAnalysisProviderModuleStateChange = LanguageProviderModuleStateChange;

export class LanguageAnalysisProviderModuleRegistry extends LanguageProviderModuleRegistry<LanguageAnalysisProvider> {}

export class LanguageAnalysisProviderModuleHost extends LanguageProviderModuleHost<LanguageAnalysisProvider> {
  constructor(modules: LanguageAnalysisProviderModuleRegistry, providers: LanguageAnalysisProviderRegistry) {
    super(modules, providers);
  }
}

export function normalizeLanguageAnalysisProviderModuleCatalog(value: unknown): LanguageAnalysisProviderModuleCatalog {
  return normalizeLanguageProviderModuleCatalog(value);
}
