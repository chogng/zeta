import { DisposableOwner, type IDisposable } from "../../../base/common/lifecycle.js";
import { type TextModel } from "../model/textModel.js";
import { registerBuiltinLanguageConfigurations } from "../languages/languageBuiltinConfigurations.js";
import { SyntaxProviderRegistry, type SyntaxProvider } from "../languages/syntax/syntaxProviders.js";
import { SyntaxService, type SyntaxWorkerDecorator, type SyntaxWorkerFactory } from "../languages/syntax/syntaxService.js";
import { LanguageCompletionProviderRegistry, type LanguageCompletionProvider } from "../languages/completion/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../languages/completion/languageCompletionService.js";
import { LanguageConfigurationRegistry, type LanguageConfiguration, type LanguageConfigurationRegistrationOptions, type LanguageConfigurationSource } from "../languages/languageConfiguration.js";
import { registerBuiltinLanguageDescriptions } from "../languages/languageBuiltinDescriptions.js";
import { LanguageRegistry, type LanguageDescription, type LanguageRegistrationOptions } from "../languages/languageRegistry.js";
import type { TextResourceLanguageInput } from "../../../platform/language/common/textResourceLanguage.js";
import { createLanguageLexicalSyntaxProvider } from "../languages/languageLexicalSyntaxProvider.js";
import { createLanguageWordCompletionProvider } from "../languages/completion/languageWordCompletionProvider.js";
import { CodeActionService, type LanguageCodeActionProvider } from "../../contrib/codeAction/common/codeAction.js";
import { CodeLensService, type LanguageCodeLensProvider } from "../../contrib/codelens/common/codelens.js";
import { DocumentSymbolService, type DocumentSymbolServiceOptions, type LanguageDocumentSymbolProvider } from "../../contrib/documentSymbols/common/documentSymbols.js";
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
import { LanguageNavigationService, type LanguageDeclarationProvider, type LanguageDefinitionProvider, type LanguageImplementationProvider, type LanguageReferenceProvider, type LanguageTypeDefinitionProvider } from "../../contrib/gotoSymbol/common/languageNavigation.js";
import { type URI } from "../../../base/common/uri.js";
import { WorkspaceSymbolService, type LanguageWorkspaceSymbolProvider } from "../languages/workspaceSymbols.js";
import { LanguageHierarchyService, type LanguageCallHierarchyProvider, type LanguageTypeHierarchyProvider } from "../../contrib/callHierarchy/common/languageHierarchy.js";

/** Language provider boundary consumed by browser and host adapters. */
export interface ILanguageFeaturesService extends IDisposable {
  readonly languages: LanguageRegistry;
  readonly configurations: LanguageConfigurationSource;
  registerLanguage(description: LanguageDescription, options?: LanguageRegistrationOptions): IDisposable;
  resolveLanguageId(input: TextResourceLanguageInput): string | undefined;
  registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options?: LanguageConfigurationRegistrationOptions): IDisposable;
  registerSyntaxProvider(provider: SyntaxProvider): IDisposable;
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
  registerDefinitionProvider(provider: LanguageDefinitionProvider): IDisposable;
  registerDeclarationProvider(provider: LanguageDeclarationProvider): IDisposable;
  registerImplementationProvider(provider: LanguageImplementationProvider): IDisposable;
  registerTypeDefinitionProvider(provider: LanguageTypeDefinitionProvider): IDisposable;
  registerReferenceProvider(provider: LanguageReferenceProvider): IDisposable;
  registerWorkspaceSymbolProvider(provider: LanguageWorkspaceSymbolProvider): IDisposable;
  registerCallHierarchyProvider(provider: LanguageCallHierarchyProvider): IDisposable;
  registerTypeHierarchyProvider(provider: LanguageTypeHierarchyProvider): IDisposable;
  createSyntaxService(model: TextModel, options?: SyntaxFeaturesOptions): SyntaxService;
  createCompletionService(model: TextModel, options?: LanguageCompletionFeaturesOptions): LanguageCompletionService;
  createCodeActionService(model: TextModel, resource: URI): CodeActionService;
  createCodeLensService(model: TextModel): CodeLensService;
  createDocumentSymbolService(model: TextModel, options?: DocumentSymbolServiceOptions): DocumentSymbolService;
  createFormatService(model: TextModel, resource?: URI): FormatService;
  createGotoSymbolService(model: TextModel, options?: DocumentSymbolServiceOptions): GotoSymbolService;
  createHoverService(model: TextModel, resource?: URI): HoverService;
  createInlayHintsService(model: TextModel, resource?: URI): InlayHintsService;
  createInlineCompletionsService(model: TextModel): InlineCompletionsService;
  createLinkedEditingService(model: TextModel, resource?: URI): LinkedEditingService;
  createLinkService(model: TextModel): LinkService;
  createParameterHintsService(model: TextModel, resource?: URI): ParameterHintsService;
  createRenameService(model: TextModel, resource: URI): RenameService;
  createColorService(model: TextModel): ColorService;
  createLanguageNavigationService(model: TextModel, resource: URI): LanguageNavigationService;
  createWorkspaceSymbolService(): WorkspaceSymbolService;
  createLanguageHierarchyService(model: TextModel, resource: URI): LanguageHierarchyService;
}

export interface SyntaxFeaturesOptions {
  readonly workerFactory?: SyntaxWorkerFactory;
  readonly workerDecorator?: SyntaxWorkerDecorator;
}

export interface LanguageCompletionFeaturesOptions {
  readonly resource?: URI;
  readonly workerFactory?: LanguageCompletionWorkerFactory;
}

/** Provides built-in language configuration and provider registries. */
export class LanguageFeaturesService extends DisposableOwner implements ILanguageFeaturesService {
  readonly languages: LanguageRegistry;
  readonly configurations: LanguageConfigurationRegistry;
  private readonly syntaxProviders: SyntaxProviderRegistry;
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
  private readonly definitionProviders: LanguageFeatureProviderRegistry<LanguageDefinitionProvider>;
  private readonly declarationProviders: LanguageFeatureProviderRegistry<LanguageDeclarationProvider>;
  private readonly implementationProviders: LanguageFeatureProviderRegistry<LanguageImplementationProvider>;
  private readonly typeDefinitionProviders: LanguageFeatureProviderRegistry<LanguageTypeDefinitionProvider>;
  private readonly referenceProviders: LanguageFeatureProviderRegistry<LanguageReferenceProvider>;
  private readonly workspaceSymbolProviders: LanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>;
  private readonly callHierarchyProviders: LanguageFeatureProviderRegistry<LanguageCallHierarchyProvider>;
  private readonly typeHierarchyProviders: LanguageFeatureProviderRegistry<LanguageTypeHierarchyProvider>;

  constructor() {
    super();
    this.languages = this.own(new LanguageRegistry());
    this.own(registerBuiltinLanguageDescriptions(this.languages));
    this.configurations = this.own(new LanguageConfigurationRegistry());
    this.own(registerBuiltinLanguageConfigurations(this.configurations));
    this.syntaxProviders = this.own(new SyntaxProviderRegistry());
    this.own(this.syntaxProviders.register(createLanguageLexicalSyntaxProvider({ languageConfigurations: this.configurations })));
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
    this.definitionProviders = this.own(new LanguageFeatureProviderRegistry());
    this.declarationProviders = this.own(new LanguageFeatureProviderRegistry());
    this.implementationProviders = this.own(new LanguageFeatureProviderRegistry());
    this.typeDefinitionProviders = this.own(new LanguageFeatureProviderRegistry());
    this.referenceProviders = this.own(new LanguageFeatureProviderRegistry());
    this.workspaceSymbolProviders = this.own(new LanguageFeatureProviderRegistry());
    this.callHierarchyProviders = this.own(new LanguageFeatureProviderRegistry());
    this.typeHierarchyProviders = this.own(new LanguageFeatureProviderRegistry());
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

  registerSyntaxProvider(provider: SyntaxProvider): IDisposable {
    return this.syntaxProviders.register(provider);
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

  registerDefinitionProvider(provider: LanguageDefinitionProvider): IDisposable {
    return this.definitionProviders.register(provider);
  }

  registerDeclarationProvider(provider: LanguageDeclarationProvider): IDisposable {
    return this.declarationProviders.register(provider);
  }

  registerImplementationProvider(provider: LanguageImplementationProvider): IDisposable {
    return this.implementationProviders.register(provider);
  }

  registerTypeDefinitionProvider(provider: LanguageTypeDefinitionProvider): IDisposable {
    return this.typeDefinitionProviders.register(provider);
  }

  registerReferenceProvider(provider: LanguageReferenceProvider): IDisposable {
    return this.referenceProviders.register(provider);
  }

  registerWorkspaceSymbolProvider(provider: LanguageWorkspaceSymbolProvider): IDisposable {
    return this.workspaceSymbolProviders.register(provider);
  }

  registerCallHierarchyProvider(provider: LanguageCallHierarchyProvider): IDisposable { return this.callHierarchyProviders.register(provider); }
  registerTypeHierarchyProvider(provider: LanguageTypeHierarchyProvider): IDisposable { return this.typeHierarchyProviders.register(provider); }

  createSyntaxService(model: TextModel, options: SyntaxFeaturesOptions = {}): SyntaxService {
    return new SyntaxService(model, this.syntaxProviders, {
      ...(options.workerFactory ? { workerFactory: options.workerFactory } : {}),
      ...(options.workerDecorator ? { workerDecorator: options.workerDecorator } : {}),
    });
  }

  createCompletionService(model: TextModel, options: LanguageCompletionFeaturesOptions = {}): LanguageCompletionService {
    return new LanguageCompletionService(model, this.completionProviders, {
      ...(options.resource ? { resource: options.resource } : {}),
      ...(options.workerFactory ? { workerFactory: options.workerFactory } : {}),
    });
  }

  createCodeActionService(model: TextModel, resource: URI): CodeActionService {
    return new CodeActionService(model, resource, this.codeActionProviders);
  }

  createCodeLensService(model: TextModel): CodeLensService {
    return new CodeLensService(model, this.codeLensProviders);
  }

  createDocumentSymbolService(model: TextModel, options: DocumentSymbolServiceOptions = {}): DocumentSymbolService {
    return new DocumentSymbolService(model, this.documentSymbolProviders, options);
  }

  createFormatService(model: TextModel, resource?: URI): FormatService {
    return new FormatService(model, this.formattingProviders, resource);
  }

  createGotoSymbolService(model: TextModel, options: DocumentSymbolServiceOptions = {}): GotoSymbolService {
    return new GotoSymbolService(this.createDocumentSymbolService(model, options));
  }

  createHoverService(model: TextModel, resource?: URI): HoverService {
    return new HoverService(model, this.hoverProviders, resource);
  }

  createInlayHintsService(model: TextModel, resource?: URI): InlayHintsService {
    return new InlayHintsService(model, this.inlayHintsProviders, resource);
  }

  createInlineCompletionsService(model: TextModel): InlineCompletionsService {
    return new InlineCompletionsService(model, this.inlineCompletionsProviders);
  }

  createLinkedEditingService(model: TextModel, resource?: URI): LinkedEditingService {
    return new LinkedEditingService(model, this.linkedEditingProviders, resource);
  }

  createLinkService(model: TextModel): LinkService {
    return new LinkService(model, this.linkProviders);
  }

  createParameterHintsService(model: TextModel, resource?: URI): ParameterHintsService {
    return new ParameterHintsService(model, this.parameterHintsProviders, resource);
  }

  createRenameService(model: TextModel, resource: URI): RenameService {
    return new RenameService(model, resource, this.renameProviders);
  }

  createColorService(model: TextModel): ColorService {
    return new ColorService(model, this.colorProviders);
  }

  createLanguageNavigationService(model: TextModel, resource: URI): LanguageNavigationService {
    return new LanguageNavigationService(model, resource, {
      definitions: this.definitionProviders,
      declarations: this.declarationProviders,
      implementations: this.implementationProviders,
      typeDefinitions: this.typeDefinitionProviders,
      references: this.referenceProviders,
    });
  }

  createWorkspaceSymbolService(): WorkspaceSymbolService {
    return new WorkspaceSymbolService(this.workspaceSymbolProviders);
  }

  createLanguageHierarchyService(model: TextModel, resource: URI): LanguageHierarchyService {
    return new LanguageHierarchyService(model, resource, this.callHierarchyProviders, this.typeHierarchyProviders);
  }
}
