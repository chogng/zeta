import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";
import { type TextModel } from "../../../../editor/alpha/common/textModel.js";
import { registerAlphaBuiltinLanguageConfigurations } from "../../../../editor/alpha/language/common/languageBuiltinConfigurations.js";
import { LanguageAnalysisProviderRegistry, type LanguageAnalysisProvider } from "../../../../editor/alpha/language/common/languageAnalysisProviders.js";
import { LanguageAnalysisService, type LanguageAnalysisWorkerFactory } from "../../../../editor/alpha/language/common/languageAnalysisService.js";
import { LanguageCompletionProviderRegistry, type LanguageCompletionProvider } from "../../../../editor/alpha/language/common/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../../../../editor/alpha/language/common/languageCompletionService.js";
import { LanguageConfigurationRegistry, type LanguageConfiguration, type LanguageConfigurationRegistrationOptions, type LanguageConfigurationSource } from "../../../../editor/alpha/language/common/languageConfiguration.js";
import { createLanguageLexicalAnalysisProvider } from "../../../../editor/alpha/language/common/languageLexicalAnalysisProvider.js";
import { createLanguageWordCompletionProvider } from "../../../../editor/alpha/language/common/languageWordCompletionProvider.js";

/**
 * Language-provider boundary consumed by editor hosts.
 *
 * Implementations own language configuration and provider registration. A caller
 * owns each returned per-document service, so closing an editor cannot dispose
 * shared language registrations.
 */
export interface ILanguageFeaturesService {
  readonly configurations: LanguageConfigurationSource;
  registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options?: LanguageConfigurationRegistrationOptions): IDisposable;
  registerAnalysisProvider(provider: LanguageAnalysisProvider): IDisposable;
  registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable;
  createAnalysisService(model: TextModel, options?: LanguageAnalysisFeaturesOptions): LanguageAnalysisService;
  createCompletionService(model: TextModel, options?: LanguageCompletionFeaturesOptions): LanguageCompletionService;
}

export const ILanguageFeaturesService = createServiceIdentifier<ILanguageFeaturesService>("languageFeaturesService");

export interface LanguageAnalysisFeaturesOptions {
  readonly workerFactory?: LanguageAnalysisWorkerFactory;
}

export interface LanguageCompletionFeaturesOptions {
  readonly workerFactory?: LanguageCompletionWorkerFactory;
}

/** Default in-process language registrations used until an extension or LSP host replaces them. */
export class LanguageFeaturesService extends DisposableOwner implements ILanguageFeaturesService {
  readonly configurations: LanguageConfigurationRegistry;
  private readonly analysisProviders: LanguageAnalysisProviderRegistry;
  private readonly completionProviders: LanguageCompletionProviderRegistry;

  constructor() {
    super();
    this.configurations = this.own(new LanguageConfigurationRegistry());
    this.own(registerAlphaBuiltinLanguageConfigurations(this.configurations));
    this.analysisProviders = this.own(new LanguageAnalysisProviderRegistry());
    this.own(this.analysisProviders.register(createLanguageLexicalAnalysisProvider({ languageConfigurations: this.configurations })));
    this.completionProviders = this.own(new LanguageCompletionProviderRegistry());
    this.own(this.completionProviders.register(createLanguageWordCompletionProvider()));
  }

  registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
    return this.configurations.register(languageId, configuration, options);
  }

  registerAnalysisProvider(provider: LanguageAnalysisProvider): IDisposable {
    return this.analysisProviders.register(provider);
  }

  registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable {
    return this.completionProviders.register(provider);
  }

  createAnalysisService(model: TextModel, options: LanguageAnalysisFeaturesOptions = {}): LanguageAnalysisService {
    return new LanguageAnalysisService(model, this.analysisProviders, {
      ...(options.workerFactory ? { workerFactory: options.workerFactory } : {}),
    });
  }

  createCompletionService(model: TextModel, options: LanguageCompletionFeaturesOptions = {}): LanguageCompletionService {
    return new LanguageCompletionService(model, this.completionProviders, {
      ...(options.workerFactory ? { workerFactory: options.workerFactory } : {}),
    });
  }
}
