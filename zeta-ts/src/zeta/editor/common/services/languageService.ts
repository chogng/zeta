import { DisposableOwner, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import { runWithBufferedEvents } from "../../../base/common/event.js";
import { type TextModel } from "../model/textModel.js";
import { registerBuiltinLanguageConfigurations } from "../languages/languageBuiltinConfigurations.js";
import { SyntaxProviderRegistry, type SyntaxProvider } from "../languages/syntax/syntaxProviders.js";
import { SyntaxService, type SyntaxWorkerDecorator, type SyntaxWorkerFactory } from "../languages/syntax/syntaxService.js";
import { LanguageCompletionProviderRegistry, type LanguageCompletionProvider, type LanguageCompletionProviderRegistration } from "../languages/completion/languageCompletionProviders.js";
import { LanguageCompletionService, type LanguageCompletionWorkerFactory } from "../languages/completion/languageCompletionService.js";
import { LanguageConfigurationRegistry, type LanguageConfiguration, type LanguageConfigurationContributionInput, type LanguageConfigurationRegistration, type LanguageConfigurationRegistrationOptions, type LanguageConfigurationSource } from "../languages/languageConfiguration.js";
import { registerBuiltinLanguageDescriptions } from "../languages/languageBuiltinDescriptions.js";
import { LanguageRegistry, type LanguageDescription, type LanguageDescriptionContribution, type LanguageDescriptionRegistration, type LanguageRegistrationOptions } from "../languages/languageRegistry.js";
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
import { LanguageFeatureProviderRegistry, type LanguageFeatureProviderRegistration } from "../languages/languageFeatureRegistry.js";
import { LanguageNavigationService, type LanguageDeclarationProvider, type LanguageDefinitionProvider, type LanguageImplementationProvider, type LanguageReferenceProvider, type LanguageTypeDefinitionProvider } from "../../contrib/gotoSymbol/common/languageNavigation.js";
import { type URI } from "../../../base/common/uri.js";
import { WorkspaceSymbolService, type LanguageWorkspaceSymbolProvider } from "../languages/workspaceSymbols.js";
import { LanguageHierarchyService, type LanguageCallHierarchyProvider, type LanguageTypeHierarchyProvider } from "../../contrib/callHierarchy/common/languageHierarchy.js";
import { SemanticTokensService, type LanguageSemanticTokensProvider } from "../../contrib/semanticTokens/common/semanticTokens.js";
import { FoldingRangeService, type LanguageFoldingRangeProvider } from "../../contrib/folding/common/folding.js";

/** Language provider boundary consumed by browser and host adapters. */
export interface ILanguageFeaturesService extends IDisposable {
	readonly languages: LanguageRegistry;
	readonly configurations: LanguageConfigurationSource;
	registerLanguage(description: LanguageDescription, options?: LanguageRegistrationOptions): IDisposable;
	registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration;
	resolveLanguageId(input: TextResourceLanguageInput): string | undefined;
	registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options?: LanguageConfigurationRegistrationOptions): IDisposable;
	registerLanguageConfigurations(contributions: readonly LanguageConfigurationContributionInput[]): LanguageConfigurationRegistration;
	registerSyntaxProvider(provider: SyntaxProvider): IDisposable;
	registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable;
	registerCompletionProviders(providers: readonly LanguageCompletionProvider[]): LanguageCompletionProviderRegistration;
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
	registerSemanticTokensProvider(provider: LanguageSemanticTokensProvider): IDisposable;
	registerFoldingRangeProvider(provider: LanguageFoldingRangeProvider): IDisposable;
	registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration;
	createSyntaxService(model: TextModel, options?: SyntaxFeaturesOptions): SyntaxService;
	createCompletionService(model: TextModel, options?: LanguageCompletionFeaturesOptions): LanguageCompletionService;
	createCodeActionService(model: TextModel, resource: URI): CodeActionService;
	createCodeLensService(model: TextModel, resource?: URI): CodeLensService;
	createDocumentSymbolService(model: TextModel, options?: DocumentSymbolServiceOptions): DocumentSymbolService;
	createFormatService(model: TextModel, resource?: URI): FormatService;
	createGotoSymbolService(model: TextModel, options?: DocumentSymbolServiceOptions): GotoSymbolService;
	createHoverService(model: TextModel, resource?: URI): HoverService;
	createInlayHintsService(model: TextModel, resource?: URI): InlayHintsService;
	createInlineCompletionsService(model: TextModel): InlineCompletionsService;
	createLinkedEditingService(model: TextModel, resource?: URI): LinkedEditingService;
	createLinkService(model: TextModel, resource?: URI): LinkService;
	createParameterHintsService(model: TextModel, resource?: URI): ParameterHintsService;
	createRenameService(model: TextModel, resource: URI): RenameService;
	createColorService(model: TextModel, resource?: URI): ColorService;
	createLanguageNavigationService(model: TextModel, resource: URI): LanguageNavigationService;
	createWorkspaceSymbolService(): WorkspaceSymbolService;
	createLanguageHierarchyService(model: TextModel, resource: URI): LanguageHierarchyService;
	createSemanticTokensService(model: TextModel, resource?: URI): SemanticTokensService;
	createFoldingRangeService(model: TextModel, resource?: URI): FoldingRangeService;
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
	private readonly semanticTokensProviders: LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>;
	private readonly foldingRangeProviders: LanguageFeatureProviderRegistry<LanguageFoldingRangeProvider>;

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
		this.semanticTokensProviders = this.own(new LanguageFeatureProviderRegistry());
		this.foldingRangeProviders = this.own(new LanguageFeatureProviderRegistry());
	}

	registerLanguage(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
		return this.languages.register(description, options);
	}

	registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration {
		return this.languages.registerMany(contributions);
	}

	resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
		return this.languages.resolveLanguageId(input);
	}

	registerLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
		return this.configurations.register(languageId, configuration, options);
	}

	registerLanguageConfigurations(contributions: readonly LanguageConfigurationContributionInput[]): LanguageConfigurationRegistration {
		return this.configurations.registerMany(contributions);
	}

	registerSyntaxProvider(provider: SyntaxProvider): IDisposable {
		return this.syntaxProviders.register(provider);
	}

	registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable {
		return this.completionProviders.register(provider);
	}

	registerCompletionProviders(providers: readonly LanguageCompletionProvider[]): LanguageCompletionProviderRegistration {
		return this.completionProviders.registerGroup(providers);
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
	registerSemanticTokensProvider(provider: LanguageSemanticTokensProvider): IDisposable { return this.semanticTokensProviders.register(provider); }
	registerFoldingRangeProvider(provider: LanguageFoldingRangeProvider): IDisposable { return this.foldingRangeProviders.register(provider); }

	registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration {
		const completions = this.completionProviders.registerGroup([]);
		const hovers = this.hoverProviders.registerGroup([]);
		const formatting = this.formattingProviders.registerGroup([]);
		const inlayHints = this.inlayHintsProviders.registerGroup([]);
		const linkedEditing = this.linkedEditingProviders.registerGroup([]);
		const parameterHints = this.parameterHintsProviders.registerGroup([]);
		const registrations = { completions, hovers, formatting, inlayHints, linkedEditing, parameterHints };
		let current = emptyProviderBatch();
		let disposed = false;
		const replace = (replacement: LanguageProviderBatch): void => {
			if (disposed) throw new ReferenceError("Language provider batch registration is already disposed");
			const next = normalizeProviderBatch(replacement);
			runWithBufferedEvents(() => {
				try {
					replaceProviderRegistrations(registrations, next);
				} catch (error) {
					replaceProviderRegistrations(registrations, current);
					throw error;
				}
			});
			current = next;
		};
		replace(providers);
		const registration = toDisposable(() => {
			if (disposed) return;
			disposed = true;
			runWithBufferedEvents(() => {
				parameterHints.dispose();
				linkedEditing.dispose();
				inlayHints.dispose();
				formatting.dispose();
				hovers.dispose();
				completions.dispose();
			});
		}) as LanguageProviderBatchRegistration;
		registration.replace = replace;
		return registration;
	}

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

	createCodeLensService(model: TextModel, resource?: URI): CodeLensService {
		return new CodeLensService(model, this.codeLensProviders, resource);
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

	createLinkService(model: TextModel, resource?: URI): LinkService {
		return new LinkService(model, this.linkProviders, resource);
	}

	createParameterHintsService(model: TextModel, resource?: URI): ParameterHintsService {
		return new ParameterHintsService(model, this.parameterHintsProviders, resource);
	}

	createRenameService(model: TextModel, resource: URI): RenameService {
		return new RenameService(model, resource, this.renameProviders);
	}

	createColorService(model: TextModel, resource?: URI): ColorService {
		return new ColorService(model, this.colorProviders, resource);
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

	createSemanticTokensService(model: TextModel, resource?: URI): SemanticTokensService {
		return new SemanticTokensService(model, this.semanticTokensProviders, resource);
	}

	createFoldingRangeService(model: TextModel, resource?: URI): FoldingRangeService {
		return new FoldingRangeService(model, this.foldingRangeProviders, resource);
	}
}

interface LanguageProviderRegistrations {
	readonly completions: LanguageCompletionProviderRegistration;
	readonly hovers: LanguageFeatureProviderRegistration<LanguageHoverProvider>;
	readonly formatting: LanguageFeatureProviderRegistration<LanguageFormattingProvider>;
	readonly inlayHints: LanguageFeatureProviderRegistration<LanguageInlayHintsProvider>;
	readonly linkedEditing: LanguageFeatureProviderRegistration<LanguageLinkedEditingProvider>;
	readonly parameterHints: LanguageFeatureProviderRegistration<LanguageParameterHintsProvider>;
}

function replaceProviderRegistrations(registrations: LanguageProviderRegistrations, providers: Required<LanguageProviderBatch>): void {
	registrations.completions.replace(providers.completions);
	registrations.hovers.replace(providers.hovers);
	registrations.formatting.replace(providers.formatting);
	registrations.inlayHints.replace(providers.inlayHints);
	registrations.linkedEditing.replace(providers.linkedEditing);
	registrations.parameterHints.replace(providers.parameterHints);
}

function normalizeProviderBatch(value: LanguageProviderBatch): Required<LanguageProviderBatch> {
	if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError("Language provider batch must be an object");
	const record = value as Record<string, unknown>;
	const supported = new Set(["completions", "hovers", "formatting", "inlayHints", "linkedEditing", "parameterHints"]);
	if (Object.keys(record).some(key => !supported.has(key))) throw new TypeError("Language provider batch contains an unsupported provider kind");
	return Object.freeze({
		completions: frozenProviderArray(value.completions, "completion"),
		hovers: frozenProviderArray(value.hovers, "hover"),
		formatting: frozenProviderArray(value.formatting, "formatting"),
		inlayHints: frozenProviderArray(value.inlayHints, "Inlay Hints"),
		linkedEditing: frozenProviderArray(value.linkedEditing, "Linked Editing"),
		parameterHints: frozenProviderArray(value.parameterHints, "Parameter Hints"),
	});
}

function emptyProviderBatch(): Required<LanguageProviderBatch> {
	const empty = Object.freeze([]);
	return Object.freeze({ completions: empty, hovers: empty, formatting: empty, inlayHints: empty, linkedEditing: empty, parameterHints: empty });
}

function frozenProviderArray<T>(value: readonly T[] | undefined, owner: string): readonly T[] {
	if (value === undefined) return Object.freeze([]);
	if (!Array.isArray(value)) throw new TypeError(`Language ${owner} providers must be an array`);
	return Object.freeze([...value]);
}

/** Canonical caller-owned batch used when one runtime generation contributes several provider kinds. */
export interface LanguageProviderBatch {
	readonly completions?: readonly LanguageCompletionProvider[];
	readonly hovers?: readonly LanguageHoverProvider[];
	readonly formatting?: readonly LanguageFormattingProvider[];
	readonly inlayHints?: readonly LanguageInlayHintsProvider[];
	readonly linkedEditing?: readonly LanguageLinkedEditingProvider[];
	readonly parameterHints?: readonly LanguageParameterHintsProvider[];
}

export interface LanguageProviderBatchRegistration extends IDisposable {
	replace(providers: LanguageProviderBatch): void;
}
