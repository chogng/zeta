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
import { LanguageConfigurationRegistrationOptions } from '../../common/languages/ownedLanguageConfigurationContributions.js';
import type { LanguageDescriptionChangeEvent, LanguageDescriptionContribution, LanguageDescriptionRegistration } from '../../common/languages/languageRegistry.js';
import { LanguageDiagnosticSeverity } from '../../common/languages/languageResults.js';
import type { SyntaxProvider } from '../../common/languages/syntax/syntaxProviders.js';
import type { LanguageWorkspaceSymbolProvider } from '../../common/languages/workspaceSymbols.js';
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
	readonly registerLanguageConfiguration: typeof registerLanguageConfiguration;
	readonly registerProviderBatch: typeof registerProviderBatch;
	readonly registerSyntaxProvider: typeof registerSyntaxProvider;
	readonly registerCompletionProvider: typeof registerCompletionProvider;
	readonly registerLanguageCodeActionProvider: typeof registerLanguageCodeActionProvider;
	readonly registerLanguageCodeLensProvider: typeof registerLanguageCodeLensProvider;
	readonly registerLanguageDocumentSymbolProvider: typeof registerLanguageDocumentSymbolProvider;
	readonly registerFormattingProvider: typeof registerFormattingProvider;
	readonly registerLanguageHoverProvider: typeof registerLanguageHoverProvider;
	readonly registerLanguageInlayHintsProvider: typeof registerLanguageInlayHintsProvider;
	readonly registerLanguageInlineCompletionsProvider: typeof registerLanguageInlineCompletionsProvider;
	readonly registerLinkedEditingProvider: typeof registerLinkedEditingProvider;
	readonly registerLanguageLinkProvider: typeof registerLanguageLinkProvider;
	readonly registerParameterHintsProvider: typeof registerParameterHintsProvider;
	readonly registerLanguageRenameProvider: typeof registerLanguageRenameProvider;
	readonly registerLanguageColorProvider: typeof registerLanguageColorProvider;
	readonly registerLanguageDefinitionProvider: typeof registerLanguageDefinitionProvider;
	readonly registerLanguageDeclarationProvider: typeof registerLanguageDeclarationProvider;
	readonly registerLanguageImplementationProvider: typeof registerLanguageImplementationProvider;
	readonly registerLanguageTypeDefinitionProvider: typeof registerLanguageTypeDefinitionProvider;
	readonly registerLanguageReferenceProvider: typeof registerLanguageReferenceProvider;
	readonly registerWorkspaceSymbolProvider: typeof registerWorkspaceSymbolProvider;
	readonly registerCallHierarchyProvider: typeof registerCallHierarchyProvider;
	readonly registerTypeHierarchyProvider: typeof registerTypeHierarchyProvider;
	readonly registerSemanticTokensProvider: typeof registerSemanticTokensProvider;
	readonly registerLanguageFoldingRangeProvider: typeof registerLanguageFoldingRangeProvider;
	readonly registerLanguageSelectionRangeProvider: typeof registerLanguageSelectionRangeProvider;
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

export function registerLanguageConfiguration(languageId: string, configuration: LanguageConfigurationInput, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
	return StandaloneServices.get().languageConfigurationService.register(languageId, configuration, options);
}

export function registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration {
	return StandaloneServices.get().languageFeaturesService.registerProviderBatch(providers);
}

export function registerSyntaxProvider(provider: SyntaxProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.syntaxProvider.register(provider); }
export function registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.completionProvider.register(provider); }
export function registerLanguageCodeActionProvider(provider: LanguageCodeActionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.codeActionProvider.register(provider); }
export function registerLanguageCodeLensProvider(provider: LanguageCodeLensProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.codeLensProvider.register(provider); }
export function registerLanguageDocumentSymbolProvider(provider: LanguageDocumentSymbolProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.documentSymbolProvider.register(provider); }
export function registerFormattingProvider(provider: LanguageFormattingProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.formattingProvider.register(provider); }
export function registerLanguageHoverProvider(provider: LanguageHoverProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.hoverProvider.register(provider); }
export function registerLanguageInlayHintsProvider(provider: LanguageInlayHintsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.inlayHintsProvider.register(provider); }
export function registerLanguageInlineCompletionsProvider(provider: LanguageInlineCompletionsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.inlineCompletionsProvider.register(provider); }
export function registerLinkedEditingProvider(provider: LanguageLinkedEditingProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.linkedEditingProvider.register(provider); }
export function registerLanguageLinkProvider(provider: LanguageLinkProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.linkProvider.register(provider); }
export function registerParameterHintsProvider(provider: LanguageParameterHintsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.parameterHintsProvider.register(provider); }
export function registerLanguageRenameProvider(provider: LanguageRenameProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.renameProvider.register(provider); }
export function registerLanguageColorProvider(provider: LanguageColorProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.colorProvider.register(provider); }
export function registerLanguageDefinitionProvider(provider: LanguageDefinitionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.definitionProvider.register(provider); }
export function registerLanguageDeclarationProvider(provider: LanguageDeclarationProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.declarationProvider.register(provider); }
export function registerLanguageImplementationProvider(provider: LanguageImplementationProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.implementationProvider.register(provider); }
export function registerLanguageTypeDefinitionProvider(provider: LanguageTypeDefinitionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.typeDefinitionProvider.register(provider); }
export function registerLanguageReferenceProvider(provider: LanguageReferenceProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.referenceProvider.register(provider); }
export function registerWorkspaceSymbolProvider(provider: LanguageWorkspaceSymbolProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.workspaceSymbolProvider.register(provider); }
export function registerCallHierarchyProvider(provider: LanguageCallHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.callHierarchyProvider.register(provider); }
export function registerTypeHierarchyProvider(provider: LanguageTypeHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.typeHierarchyProvider.register(provider); }
export function registerSemanticTokensProvider(provider: LanguageSemanticTokensProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.semanticTokensProvider.register(provider); }
export function registerLanguageFoldingRangeProvider(provider: LanguageFoldingRangeProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.foldingRangeProvider.register(provider); }
export function registerLanguageSelectionRangeProvider(provider: LanguageSelectionRangeProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.selectionRangeProvider.register(provider); }
export function registerDocumentHighlightProvider(selector: LanguageSelector, provider: languages.DocumentHighlightProvider): IDisposable {
	return StandaloneServices.get().languageFeaturesService.documentHighlightProvider.register(Object.freeze({
		languageIds: languageIdsForSelector(selector),
		provideDocumentHighlights: provider.provideDocumentHighlights.bind(provider),
	}));
}

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
		registerLanguageConfiguration,
		registerProviderBatch,
		registerSyntaxProvider,
		registerCompletionProvider,
		registerLanguageCodeActionProvider,
		registerLanguageCodeLensProvider,
		registerLanguageDocumentSymbolProvider,
		registerFormattingProvider,
		registerLanguageHoverProvider,
		registerLanguageInlayHintsProvider,
		registerLanguageInlineCompletionsProvider,
		registerLinkedEditingProvider,
		registerLanguageLinkProvider,
		registerParameterHintsProvider,
		registerLanguageRenameProvider,
		registerLanguageColorProvider,
		registerLanguageDefinitionProvider,
		registerLanguageDeclarationProvider,
		registerLanguageImplementationProvider,
		registerLanguageTypeDefinitionProvider,
		registerLanguageReferenceProvider,
		registerWorkspaceSymbolProvider,
		registerCallHierarchyProvider,
		registerTypeHierarchyProvider,
		registerSemanticTokensProvider,
		registerLanguageFoldingRangeProvider,
		registerLanguageSelectionRangeProvider,
		registerDocumentHighlightProvider,
		registerMultiDocumentHighlightProvider,
	});
}
