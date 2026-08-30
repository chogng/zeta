import type { Event } from '../../../base/common/event.js';
import type { IDisposable } from '../../../base/common/lifecycle.js';
import type { TextResourceLanguageInput } from '../../../platform/language/common/textResourceLanguage.js';
import { RGBA8 } from '../../common/core/misc/rgba.js';
import { LanguageCompletionInsertTextFormat, LanguageCompletionItemKind } from '../../common/languages/completion/languageCompletions.js';
import { LanguageCompletionTriggerKind, type LanguageCompletionProvider } from '../../common/languages/completion/languageCompletionProviders.js';
import { DocumentHighlightKind, type MultiDocumentHighlightProvider } from '../../common/languages.js';
import * as languages from '../../common/languages.js';
import { selectLanguageIds, type LanguageSelector } from '../../common/languageSelector.js';
import { LanguageConfigurationInput } from '../../common/languages/languageConfiguration.js';
import type { ILanguageExtensionPoint } from '../../common/languages/language.js';
import type { LanguageDescriptionChangeEvent, LanguageDescriptionContribution, LanguageDescriptionRegistration } from '../../common/languages/languageRegistry.js';
import { LanguageDiagnosticSeverity } from '../../common/languages/languageResults.js';
import type { SyntaxProvider } from '../../common/languages/syntax/syntaxProviders.js';
import type { LanguageWorkspaceSymbolProvider } from '../../common/languages/workspaceSymbols.js';
import type { LanguageFeatureProviderMetadata } from '../../common/ownedLanguageFeatureProviderRegistry.js';
import type { LanguageCallHierarchyProvider, LanguageTypeHierarchyProvider } from '../../contrib/callHierarchy/common/languageHierarchy.js';
import type { LanguageCodeActionProvider } from '../../contrib/codeAction/common/languageCodeActions.js';
import type { LanguageCodeLensProvider } from '../../contrib/codelens/common/languageCodeLenses.js';
import type { LanguageColorProvider } from '../../contrib/colorPicker/common/languageColors.js';
import type { LanguageDocumentSymbolProvider } from '../../contrib/documentSymbols/common/languageDocumentSymbols.js';
import type { LanguageFoldingRangeProvider } from '../../contrib/folding/common/languageFoldingRanges.js';
import type { LanguageFormattingProvider } from '../../contrib/format/common/formatCommands.js';
import type { LanguageDeclarationProvider, LanguageDefinitionProvider, LanguageImplementationProvider, LanguageReferenceProvider, LanguageTypeDefinitionProvider } from '../../contrib/gotoSymbol/common/languageNavigation.js';
import type { LanguageHoverProvider } from '../../contrib/hover/common/hover.js';
import type { LanguageInlayHintsProvider } from '../../contrib/inlayHints/common/languageInlayHints.js';
import type { LanguageInlineCompletionsProvider } from '../../contrib/inlineCompletions/common/inlineCompletions.js';
import type { LanguageLinkedEditingProvider } from '../../contrib/linkedEditing/common/languageLinkedEditing.js';
import type { LanguageLinkProvider } from '../../contrib/links/common/languageLinks.js';
import type { LanguageParameterHintsProvider } from '../../contrib/parameterHints/common/languageParameterHints.js';
import type { LanguageRenameProvider } from '../../contrib/rename/common/languageRename.js';
import type { LanguageSelectionRangeProvider } from '../../contrib/smartSelect/common/selectionRanges.js';
import type { LanguageSemanticTokensProvider } from '../../contrib/semanticTokens/common/semanticTokens.js';
import type { LanguageProviderBatch, LanguageProviderBatchRegistration } from '../../common/services/languageFeatures.js';
import { StandaloneServices } from './standaloneServices.js';

type StandaloneLanguageProvider<TProvider extends LanguageFeatureProviderMetadata> = Omit<TProvider, keyof LanguageFeatureProviderMetadata>;

export interface IStandaloneLanguagesApi {
	readonly LanguageCompletionInsertTextFormat: typeof LanguageCompletionInsertTextFormat;
	readonly LanguageCompletionItemKind: typeof LanguageCompletionItemKind;
	readonly LanguageCompletionTriggerKind: typeof LanguageCompletionTriggerKind;
	readonly LanguageDiagnosticSeverity: typeof LanguageDiagnosticSeverity;
	readonly DocumentHighlightKind: typeof DocumentHighlightKind;
	readonly RGBA8: typeof RGBA8;
	readonly register: typeof register;
	readonly registerLanguages: typeof registerLanguages;
	readonly resolveLanguageId: typeof resolveLanguageId;
	readonly onDidChangeLanguages: Event<LanguageDescriptionChangeEvent>;
	readonly setLanguageConfiguration: typeof setLanguageConfiguration;
	readonly registerProviderBatch: typeof registerProviderBatch;
	readonly registerSyntaxProvider: typeof registerSyntaxProvider;
	readonly registerCompletionItemProvider: typeof registerCompletionItemProvider;
	readonly registerCodeActionProvider: typeof registerCodeActionProvider;
	readonly registerCodeLensProvider: typeof registerCodeLensProvider;
	readonly registerDocumentSymbolProvider: typeof registerDocumentSymbolProvider;
	readonly registerDocumentFormattingEditProvider: typeof registerDocumentFormattingEditProvider;
	readonly registerDocumentRangeFormattingEditProvider: typeof registerDocumentRangeFormattingEditProvider;
	readonly registerOnTypeFormattingEditProvider: typeof registerOnTypeFormattingEditProvider;
	readonly registerHoverProvider: typeof registerHoverProvider;
	readonly registerInlayHintsProvider: typeof registerInlayHintsProvider;
	readonly registerInlineCompletionsProvider: typeof registerInlineCompletionsProvider;
	readonly registerLinkedEditingRangeProvider: typeof registerLinkedEditingRangeProvider;
	readonly registerLinkProvider: typeof registerLinkProvider;
	readonly registerSignatureHelpProvider: typeof registerSignatureHelpProvider;
	readonly registerRenameProvider: typeof registerRenameProvider;
	readonly registerColorProvider: typeof registerColorProvider;
	readonly registerDefinitionProvider: typeof registerDefinitionProvider;
	readonly registerDeclarationProvider: typeof registerDeclarationProvider;
	readonly registerImplementationProvider: typeof registerImplementationProvider;
	readonly registerTypeDefinitionProvider: typeof registerTypeDefinitionProvider;
	readonly registerReferenceProvider: typeof registerReferenceProvider;
	readonly registerWorkspaceSymbolProvider: typeof registerWorkspaceSymbolProvider;
	readonly registerCallHierarchyProvider: typeof registerCallHierarchyProvider;
	readonly registerTypeHierarchyProvider: typeof registerTypeHierarchyProvider;
	readonly registerDocumentSemanticTokensProvider: typeof registerDocumentSemanticTokensProvider;
	readonly registerFoldingRangeProvider: typeof registerFoldingRangeProvider;
	readonly registerSelectionRangeProvider: typeof registerSelectionRangeProvider;
	readonly registerDocumentHighlightProvider: typeof registerDocumentHighlightProvider;
	readonly registerMultiDocumentHighlightProvider: typeof registerMultiDocumentHighlightProvider;
}

export function register(language: ILanguageExtensionPoint): void {
	StandaloneServices.get().languageService.registerLanguage(language);
}

export function registerLanguages(contributions: readonly LanguageDescriptionContribution[]): LanguageDescriptionRegistration {
	return StandaloneServices.get().languageService.registerLanguages(contributions);
}

export function resolveLanguageId(input: TextResourceLanguageInput): string | undefined {
	return StandaloneServices.get().languageService.resolveLanguageId(input);
}

export const onDidChangeLanguages: Event<LanguageDescriptionChangeEvent> = listener => StandaloneServices.get().languageService.languages.onDidChange(listener);

export function setLanguageConfiguration(languageId: string, configuration: LanguageConfigurationInput): IDisposable {
	return StandaloneServices.get().languageConfigurationService.register(languageId, configuration);
}

/** Zeta worker providers can still be replaced as one runtime generation. */
export function registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration {
	return StandaloneServices.get().languageFeaturesService.registerProviderBatch(providers);
}

/** Zeta snapshot tokenization and diagnostics use one worker-oriented provider contract. */
export function registerSyntaxProvider(provider: SyntaxProvider): IDisposable {
	return StandaloneServices.get().languageFeaturesService.syntaxProvider.register(provider);
}

export function registerCompletionItemProvider(languageSelector: LanguageSelector, provider: Omit<LanguageCompletionProvider, 'languageIds'>): IDisposable {
	return StandaloneServices.get().languageFeaturesService.completionProvider.register(withLanguageSelector(languageSelector, provider));
}

export function registerCodeActionProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageCodeActionProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.codeActionProvider); }
export function registerCodeLensProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageCodeLensProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.codeLensProvider); }
export function registerDocumentSymbolProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageDocumentSymbolProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.documentSymbolProvider); }
export function registerDocumentFormattingEditProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageFormattingProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.formattingProvider); }
export function registerDocumentRangeFormattingEditProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageFormattingProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.formattingProvider); }
export function registerOnTypeFormattingEditProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageFormattingProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.formattingProvider); }
export function registerHoverProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageHoverProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.hoverProvider); }
export function registerInlayHintsProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageInlayHintsProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.inlayHintsProvider); }
export function registerInlineCompletionsProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageInlineCompletionsProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.inlineCompletionsProvider); }
export function registerLinkedEditingRangeProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageLinkedEditingProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.linkedEditingProvider); }
export function registerLinkProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageLinkProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.linkProvider); }
export function registerSignatureHelpProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageParameterHintsProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.parameterHintsProvider); }
export function registerRenameProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageRenameProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.renameProvider); }
export function registerColorProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageColorProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.colorProvider); }
export function registerDefinitionProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageDefinitionProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.definitionProvider); }
export function registerDeclarationProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageDeclarationProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.declarationProvider); }
export function registerImplementationProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageImplementationProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.implementationProvider); }
export function registerTypeDefinitionProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageTypeDefinitionProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.typeDefinitionProvider); }
export function registerReferenceProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageReferenceProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.referenceProvider); }

/** Zeta workspace and hierarchy providers remain host-wide registrations. */
export function registerWorkspaceSymbolProvider(provider: LanguageWorkspaceSymbolProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.workspaceSymbolProvider.register(provider); }
export function registerCallHierarchyProvider(provider: LanguageCallHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.callHierarchyProvider.register(provider); }
export function registerTypeHierarchyProvider(provider: LanguageTypeHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.typeHierarchyProvider.register(provider); }

export function registerDocumentSemanticTokensProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageSemanticTokensProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.semanticTokensProvider); }
export function registerFoldingRangeProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageFoldingRangeProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.foldingRangeProvider); }
export function registerSelectionRangeProvider(languageSelector: LanguageSelector, provider: StandaloneLanguageProvider<LanguageSelectionRangeProvider>): IDisposable { return registerOwnedProvider(languageSelector, provider, StandaloneServices.get().languageFeaturesService.selectionRangeProvider); }

export function registerDocumentHighlightProvider(selector: LanguageSelector, provider: languages.DocumentHighlightProvider): IDisposable {
	return StandaloneServices.get().languageFeaturesService.documentHighlightProvider.register(Object.freeze({
		languageIds: languageIdsForSelector(selector),
		provideDocumentHighlights: provider.provideDocumentHighlights.bind(provider),
	}));
}

/** Zeta's cross-document highlight provider carries its selector as part of the request owner. */
export function registerMultiDocumentHighlightProvider(provider: MultiDocumentHighlightProvider): IDisposable {
	return StandaloneServices.get().languageFeaturesService.multiDocumentHighlightProvider.register(Object.freeze({
		languageIds: languageIdsForSelector(provider.selector),
		selector: provider.selector,
		provideMultiDocumentHighlights: provider.provideMultiDocumentHighlights.bind(provider),
	}));
}

function languageIdsForSelector(selector: LanguageSelector): readonly string[] {
	const result = new Set<string>();
	selectLanguageIds(selector, result);
	if (result.size === 0) throw new TypeError('Language selector must identify at least one language');
	return Object.freeze([...result]);
}

function withLanguageSelector<TProvider extends object>(selector: LanguageSelector, provider: TProvider): TProvider & { readonly languageIds: readonly string[] } {
	if (!provider || typeof provider !== 'object') throw new TypeError('Language feature provider must be an object');
	return Object.freeze({ ...provider, languageIds: languageIdsForSelector(selector) });
}

function registerOwnedProvider<TProvider extends LanguageFeatureProviderMetadata>(
	selector: LanguageSelector,
	provider: StandaloneLanguageProvider<TProvider>,
	registry: { register(provider: TProvider): IDisposable },
): IDisposable {
	return registry.register(withLanguageSelector(selector, provider) as TProvider);
}

export function createStandaloneLanguagesApi(): IStandaloneLanguagesApi {
	return Object.freeze({
		LanguageCompletionInsertTextFormat,
		LanguageCompletionItemKind,
		LanguageCompletionTriggerKind,
		LanguageDiagnosticSeverity,
		DocumentHighlightKind,
		RGBA8,
		register,
		registerLanguages,
		resolveLanguageId,
		onDidChangeLanguages,
		setLanguageConfiguration,
		registerProviderBatch,
		registerSyntaxProvider,
		registerCompletionItemProvider,
		registerCodeActionProvider,
		registerCodeLensProvider,
		registerDocumentSymbolProvider,
		registerDocumentFormattingEditProvider,
		registerDocumentRangeFormattingEditProvider,
		registerOnTypeFormattingEditProvider,
		registerHoverProvider,
		registerInlayHintsProvider,
		registerInlineCompletionsProvider,
		registerLinkedEditingRangeProvider,
		registerLinkProvider,
		registerSignatureHelpProvider,
		registerRenameProvider,
		registerColorProvider,
		registerDefinitionProvider,
		registerDeclarationProvider,
		registerImplementationProvider,
		registerTypeDefinitionProvider,
		registerReferenceProvider,
		registerWorkspaceSymbolProvider,
		registerCallHierarchyProvider,
		registerTypeHierarchyProvider,
		registerDocumentSemanticTokensProvider,
		registerFoldingRangeProvider,
		registerSelectionRangeProvider,
		registerDocumentHighlightProvider,
		registerMultiDocumentHighlightProvider,
	});
}
