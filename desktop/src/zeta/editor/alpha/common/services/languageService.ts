import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../model/textModel.js";
import { registerBuiltinLanguageConfigurations } from "../languages/languageBuiltinConfigurations.js";
import { LanguageAnalysisProviderRegistry, type LanguageAnalysisProvider } from "../languages/analysis/languageAnalysisProviders.js";
import { LanguageAnalysisService, type LanguageAnalysisWorkerFactory } from "../languages/analysis/languageAnalysisService.js";
import { LanguageCompletionProviderRegistry, type LanguageCompletionProvider } from "../languages/completion/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../languages/completion/languageCompletionService.js";
import { LanguageConfigurationRegistry, type LanguageConfiguration, type LanguageConfigurationRegistrationOptions, type LanguageConfigurationSource } from "../languages/languageConfiguration.js";
import { registerBuiltinLanguageDescriptions } from "../languages/languageBuiltinDescriptions.js";
import { LanguageRegistry, type LanguageDescription, type LanguageRegistrationOptions } from "../languages/languageRegistry.js";
import type { TextResourceLanguageInput } from "../../../../platform/language/common/textResourceLanguage.js";
import { createLanguageLexicalAnalysisProvider } from "../languages/languageLexicalAnalysisProvider.js";
import { createLanguageWordCompletionProvider } from "../languages/completion/languageWordCompletionProvider.js";
import { CodeActionService, type LanguageCodeActionProvider } from "../../contrib/codeAction/common/codeAction.js";
import { CodeLensService, type LanguageCodeLensProvider } from "../../contrib/codelens/common/codelens.js";
import { DocumentSymbolService, type LanguageDocumentSymbolProvider } from "../../contrib/documentSymbols/common/documentSymbols.js";
import { FormatService, type LanguageFormattingProvider } from "../../contrib/format/common/formatCommands.js";
import { GotoSymbolService } from "../../contrib/gotoSymbol/common/gotoSymbol.js";
import { HoverService, type LanguageHoverProvider } from "../../contrib/hover/common/hover.js";
import { InlayHintsService, type LanguageInlayHintsProvider } from "../../contrib/inlayHints/common/inlayHints.js";
import { InlineCompletionsService, type LanguageInlineCompletionsProvider } from "../../contrib/inlineCompletions/common/inlineCompletions.js";
import { LinkedEditingService, type LanguageLinkedEditingProvider } from "../../contrib/linkedEditing/common/linkedEditing.js";
import { LinkService, type LanguageLinkProvider } from "../../contrib/links/common/links.js";
import { ParameterHintsService, type LanguageParameterHintsProvider } from "../../contrib/parameterHints/common/parameterHints.js";
import { RenameService, type LanguageRenameProvider } from "../../contrib/rename/common/rename.js";
import { ColorService, type LanguageColorProvider } from "../../contrib/colorPicker/common/color.js";
import { LanguageFeatureProviderRegistry } from "../languages/languageFeatureRegistry.js";

/** Language provider boundary consumed by browser and host adapters. */
export interface ILanguageFeaturesService extends IDisposable {
  readonly languages: LanguageRegistry;
  readonly configurations: LanguageConfigurationSource;
  registerLanguage(description: LanguageDescription, options?: LanguageRegistrationOptions): IDisposable;
  resolveLanguageId(input: TextResourceLanguageInput): string | undefined;
  registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options?: LanguageConfigurationRegistrationOptions): IDisposable;
  registerAnalysisProvider(provider: LanguageAnalysisProvider): IDisposable;
  registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable;
  registerCodeActionProvider(provider: LanguageCodeActionProvider): IDisposable;
  registerCodeLensProvider(provider: LanguageCodeLensProvider): IDisposable;
  registerDocumentSymbolProvider(provider: LanguageDocumentSymbolProvider): IDisposable;
  registerFormattingProvider(provider: LanguageFormattingProvider): IDisposable;
  registerHoverProvider(provider: LanguageHoverProvider): IDisposable;
  registerInlayHintsProvider(provider: LanguageInlayHintsProvider): IDisposable;
  registerInlineCompletionsProvider(provider: LanguageInlineCompletionsProvider): IDisposable;
  registerLinkedEditingProvider(provider: LanguageLinkedEditingProvider): IDisposable;
  registerLinkProvider(provider: LanguageLinkProvider): IDisposable;
  registerParameterHintsProvider(provider: LanguageParameterHintsProvider): IDisposable;
  registerRenameProvider(provider: LanguageRenameProvider): IDisposable;
  registerColorProvider(provider: LanguageColorProvider): IDisposable;
  createAnalysisService(model: TextModel, options?: LanguageAnalysisFeaturesOptions): LanguageAnalysisService;
  createCompletionService(model: TextModel, options?: LanguageCompletionFeaturesOptions): LanguageCompletionService;
  createCodeActionService(model: TextModel): CodeActionService;
  createCodeLensService(model: TextModel): CodeLensService;
  createDocumentSymbolService(model: TextModel): DocumentSymbolService;
  createFormatService(model: TextModel): FormatService;
  createGotoSymbolService(model: TextModel): GotoSymbolService;
  createHoverService(model: TextModel): HoverService;
  createInlayHintsService(model: TextModel): InlayHintsService;
  createInlineCompletionsService(model: TextModel): InlineCompletionsService;
  createLinkedEditingService(model: TextModel): LinkedEditingService;
  createLinkService(model: TextModel): LinkService;
  createParameterHintsService(model: TextModel): ParameterHintsService;
  createRenameService(model: TextModel): RenameService;
  createColorService(model: TextModel): ColorService;
}

export interface LanguageAnalysisFeaturesOptions {
  readonly workerFactory?: LanguageAnalysisWorkerFactory;
}

export interface LanguageCompletionFeaturesOptions {
  readonly workerFactory?: LanguageCompletionWorkerFactory;
}

/** Provides built-in language configuration and provider registries. */
export class LanguageFeaturesService extends DisposableOwner implements ILanguageFeaturesService {
  readonly languages: LanguageRegistry;
  readonly configurations: LanguageConfigurationRegistry;
  private readonly analysisProviders: LanguageAnalysisProviderRegistry;
  private readonly completionProviders: LanguageCompletionProviderRegistry;
  private readonly codeActionProviders: LanguageFeatureProviderRegistry<LanguageCodeActionProvider>;
  private readonly codeLensProviders: LanguageFeatureProviderRegistry<LanguageCodeLensProvider>;
  private readonly documentSymbolProviders: LanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>;
  private readonly formattingProviders: LanguageFeatureProviderRegistry<LanguageFormattingProvider>;
  private readonly hoverProviders: LanguageFeatureProviderRegistry<LanguageHoverProvider>;
  private readonly inlayHintsProviders: LanguageFeatureProviderRegistry<LanguageInlayHintsProvider>;
  private readonly inlineCompletionsProviders: LanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>;
  private readonly linkedEditingProviders: LanguageFeatureProviderRegistry<LanguageLinkedEditingProvider>;
  private readonly linkProviders: LanguageFeatureProviderRegistry<LanguageLinkProvider>;
  private readonly parameterHintsProviders: LanguageFeatureProviderRegistry<LanguageParameterHintsProvider>;
  private readonly renameProviders: LanguageFeatureProviderRegistry<LanguageRenameProvider>;
  private readonly colorProviders: LanguageFeatureProviderRegistry<LanguageColorProvider>;

  constructor() {
    super();
    this.languages = this.own(new LanguageRegistry());
    this.own(registerBuiltinLanguageDescriptions(this.languages));
    this.configurations = this.own(new LanguageConfigurationRegistry());
    this.own(registerBuiltinLanguageConfigurations(this.configurations));
    this.analysisProviders = this.own(new LanguageAnalysisProviderRegistry());
    this.own(this.analysisProviders.register(createLanguageLexicalAnalysisProvider({ languageConfigurations: this.configurations })));
    this.completionProviders = this.own(new LanguageCompletionProviderRegistry());
    this.own(this.completionProviders.register(createLanguageWordCompletionProvider()));
    this.codeActionProviders = this.own(new LanguageFeatureProviderRegistry());
    this.codeLensProviders = this.own(new LanguageFeatureProviderRegistry());
    this.documentSymbolProviders = this.own(new LanguageFeatureProviderRegistry());
    this.formattingProviders = this.own(new LanguageFeatureProviderRegistry());
    this.hoverProviders = this.own(new LanguageFeatureProviderRegistry());
    this.inlayHintsProviders = this.own(new LanguageFeatureProviderRegistry());
    this.inlineCompletionsProviders = this.own(new LanguageFeatureProviderRegistry());
    this.linkedEditingProviders = this.own(new LanguageFeatureProviderRegistry());
    this.linkProviders = this.own(new LanguageFeatureProviderRegistry());
    this.parameterHintsProviders = this.own(new LanguageFeatureProviderRegistry());
    this.renameProviders = this.own(new LanguageFeatureProviderRegistry());
    this.colorProviders = this.own(new LanguageFeatureProviderRegistry());
  }

  registerLanguage(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
    return this.languages.register(description, options);
  }

  resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
    return this.languages.resolveLanguageId(input);
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

  registerCodeActionProvider(provider: LanguageCodeActionProvider): IDisposable {
    return this.codeActionProviders.register(provider);
  }

  registerCodeLensProvider(provider: LanguageCodeLensProvider): IDisposable {
    return this.codeLensProviders.register(provider);
  }

  registerDocumentSymbolProvider(provider: LanguageDocumentSymbolProvider): IDisposable {
    return this.documentSymbolProviders.register(provider);
  }

  registerFormattingProvider(provider: LanguageFormattingProvider): IDisposable {
    return this.formattingProviders.register(provider);
  }

  registerHoverProvider(provider: LanguageHoverProvider): IDisposable {
    return this.hoverProviders.register(provider);
  }

  registerInlayHintsProvider(provider: LanguageInlayHintsProvider): IDisposable {
    return this.inlayHintsProviders.register(provider);
  }

  registerInlineCompletionsProvider(provider: LanguageInlineCompletionsProvider): IDisposable {
    return this.inlineCompletionsProviders.register(provider);
  }

  registerLinkedEditingProvider(provider: LanguageLinkedEditingProvider): IDisposable {
    return this.linkedEditingProviders.register(provider);
  }

  registerLinkProvider(provider: LanguageLinkProvider): IDisposable {
    return this.linkProviders.register(provider);
  }

  registerParameterHintsProvider(provider: LanguageParameterHintsProvider): IDisposable {
    return this.parameterHintsProviders.register(provider);
  }

  registerRenameProvider(provider: LanguageRenameProvider): IDisposable {
    return this.renameProviders.register(provider);
  }

  registerColorProvider(provider: LanguageColorProvider): IDisposable {
    return this.colorProviders.register(provider);
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

  createCodeActionService(model: TextModel): CodeActionService {
    return new CodeActionService(model, this.codeActionProviders);
  }

  createCodeLensService(model: TextModel): CodeLensService {
    return new CodeLensService(model, this.codeLensProviders);
  }

  createDocumentSymbolService(model: TextModel): DocumentSymbolService {
    return new DocumentSymbolService(model, this.documentSymbolProviders);
  }

  createFormatService(model: TextModel): FormatService {
    return new FormatService(model, this.formattingProviders);
  }

  createGotoSymbolService(model: TextModel): GotoSymbolService {
    return new GotoSymbolService(this.createDocumentSymbolService(model));
  }

  createHoverService(model: TextModel): HoverService {
    return new HoverService(model, this.hoverProviders);
  }

  createInlayHintsService(model: TextModel): InlayHintsService {
    return new InlayHintsService(model, this.inlayHintsProviders);
  }

  createInlineCompletionsService(model: TextModel): InlineCompletionsService {
    return new InlineCompletionsService(model, this.inlineCompletionsProviders);
  }

  createLinkedEditingService(model: TextModel): LinkedEditingService {
    return new LinkedEditingService(model, this.linkedEditingProviders);
  }

  createLinkService(model: TextModel): LinkService {
    return new LinkService(model, this.linkProviders);
  }

  createParameterHintsService(model: TextModel): ParameterHintsService {
    return new ParameterHintsService(model, this.parameterHintsProviders);
  }

  createRenameService(model: TextModel): RenameService {
    return new RenameService(model, this.renameProviders);
  }

  createColorService(model: TextModel): ColorService {
    return new ColorService(model, this.colorProviders);
  }
}
