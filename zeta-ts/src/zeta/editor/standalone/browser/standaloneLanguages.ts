import type { IDisposable } from '../../../base/common/lifecycle.js';
import type { LanguageCompletionProvider } from '../../common/languages/completion/languageCompletionProviders.js';
import type { DocumentHighlightProvider, MultiDocumentHighlightProvider } from '../../common/languages/documentHighlights.js';
import type { LanguageConfiguration, LanguageConfigurationRegistrationOptions } from '../../common/languages/languageConfiguration.js';
import type { LanguageDescription, LanguageRegistrationOptions } from '../../common/languages/languageRegistry.js';
import type { SyntaxProvider } from '../../common/languages/syntax/syntaxProviders.js';
import type { LanguageWorkspaceSymbolProvider } from '../../common/languages/workspaceSymbols.js';
import type { LanguageCallHierarchyProvider, LanguageTypeHierarchyProvider } from '../../contrib/callHierarchy/common/languageHierarchy.js';
import type { LanguageCodeActionProvider } from '../../contrib/codeAction/common/codeAction.js';
import type { LanguageCodeLensProvider } from '../../contrib/codelens/common/codelens.js';
import type { LanguageColorProvider } from '../../contrib/colorPicker/common/color.js';
import type { LanguageDocumentSymbolProvider } from '../../contrib/documentSymbols/common/documentSymbols.js';
import type { LanguageFoldingRangeProvider } from '../../contrib/folding/common/folding.js';
import type { LanguageFormattingProvider } from '../../contrib/format/common/formatCommands.js';
import type { LanguageDeclarationProvider, LanguageDefinitionProvider, LanguageImplementationProvider, LanguageReferenceProvider, LanguageTypeDefinitionProvider } from '../../contrib/gotoSymbol/common/languageNavigation.js';
import type { LanguageHoverProvider } from '../../contrib/hover/common/hover.js';
import type { LanguageInlayHintsProvider } from '../../contrib/inlayHints/common/inlayHints.js';
import type { LanguageInlineCompletionsProvider } from '../../contrib/inlineCompletions/common/inlineCompletions.js';
import type { LanguageLinkedEditingProvider } from '../../contrib/linkedEditing/common/linkedEditing.js';
import type { LanguageLinkProvider } from '../../contrib/links/common/links.js';
import type { LanguageParameterHintsProvider } from '../../contrib/parameterHints/common/parameterHints.js';
import type { LanguageRenameProvider } from '../../contrib/rename/common/rename.js';
import type { LanguageSelectionRangeProvider } from '../../contrib/smartSelect/common/selectionRanges.js';
import type { LanguageSemanticTokensProvider } from '../../contrib/semanticTokens/common/semanticTokens.js';
import { StandaloneServices } from './standaloneServices.js';

export interface IStandaloneLanguagesApi {
	readonly register: typeof register;
	readonly setLanguageConfiguration: typeof setLanguageConfiguration;
	readonly registerSyntaxProvider: typeof registerSyntaxProvider;
	readonly registerCompletionProvider: typeof registerCompletionProvider;
	readonly registerCodeActionProvider: typeof registerCodeActionProvider;
	readonly registerCodeLensProvider: typeof registerCodeLensProvider;
	readonly registerDocumentSymbolProvider: typeof registerDocumentSymbolProvider;
	readonly registerFormattingProvider: typeof registerFormattingProvider;
	readonly registerHoverProvider: typeof registerHoverProvider;
	readonly registerInlayHintsProvider: typeof registerInlayHintsProvider;
	readonly registerInlineCompletionsProvider: typeof registerInlineCompletionsProvider;
	readonly registerLinkedEditingProvider: typeof registerLinkedEditingProvider;
	readonly registerLinkProvider: typeof registerLinkProvider;
	readonly registerParameterHintsProvider: typeof registerParameterHintsProvider;
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
	readonly registerSemanticTokensProvider: typeof registerSemanticTokensProvider;
	readonly registerFoldingRangeProvider: typeof registerFoldingRangeProvider;
	readonly registerSelectionRangeProvider: typeof registerSelectionRangeProvider;
	readonly registerDocumentHighlightProvider: typeof registerDocumentHighlightProvider;
	readonly registerMultiDocumentHighlightProvider: typeof registerMultiDocumentHighlightProvider;
}

export function register(description: LanguageDescription, options: LanguageRegistrationOptions = {}): IDisposable {
	return StandaloneServices.get().languageService.registerLanguage(description, options);
}

export function setLanguageConfiguration(languageId: string, configuration: LanguageConfiguration, options: LanguageConfigurationRegistrationOptions = {}): IDisposable {
	return StandaloneServices.get().languageConfigurationService.register(languageId, configuration, options);
}

export function registerSyntaxProvider(provider: SyntaxProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.syntaxProvider.register(provider); }
export function registerCompletionProvider(provider: LanguageCompletionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.completionProvider.register(provider); }
export function registerCodeActionProvider(provider: LanguageCodeActionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.codeActionProvider.register(provider); }
export function registerCodeLensProvider(provider: LanguageCodeLensProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.codeLensProvider.register(provider); }
export function registerDocumentSymbolProvider(provider: LanguageDocumentSymbolProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.documentSymbolProvider.register(provider); }
export function registerFormattingProvider(provider: LanguageFormattingProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.formattingProvider.register(provider); }
export function registerHoverProvider(provider: LanguageHoverProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.hoverProvider.register(provider); }
export function registerInlayHintsProvider(provider: LanguageInlayHintsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.inlayHintsProvider.register(provider); }
export function registerInlineCompletionsProvider(provider: LanguageInlineCompletionsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.inlineCompletionsProvider.register(provider); }
export function registerLinkedEditingProvider(provider: LanguageLinkedEditingProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.linkedEditingProvider.register(provider); }
export function registerLinkProvider(provider: LanguageLinkProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.linkProvider.register(provider); }
export function registerParameterHintsProvider(provider: LanguageParameterHintsProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.parameterHintsProvider.register(provider); }
export function registerRenameProvider(provider: LanguageRenameProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.renameProvider.register(provider); }
export function registerColorProvider(provider: LanguageColorProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.colorProvider.register(provider); }
export function registerDefinitionProvider(provider: LanguageDefinitionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.definitionProvider.register(provider); }
export function registerDeclarationProvider(provider: LanguageDeclarationProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.declarationProvider.register(provider); }
export function registerImplementationProvider(provider: LanguageImplementationProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.implementationProvider.register(provider); }
export function registerTypeDefinitionProvider(provider: LanguageTypeDefinitionProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.typeDefinitionProvider.register(provider); }
export function registerReferenceProvider(provider: LanguageReferenceProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.referenceProvider.register(provider); }
export function registerWorkspaceSymbolProvider(provider: LanguageWorkspaceSymbolProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.workspaceSymbolProvider.register(provider); }
export function registerCallHierarchyProvider(provider: LanguageCallHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.callHierarchyProvider.register(provider); }
export function registerTypeHierarchyProvider(provider: LanguageTypeHierarchyProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.typeHierarchyProvider.register(provider); }
export function registerSemanticTokensProvider(provider: LanguageSemanticTokensProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.semanticTokensProvider.register(provider); }
export function registerFoldingRangeProvider(provider: LanguageFoldingRangeProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.foldingRangeProvider.register(provider); }
export function registerSelectionRangeProvider(provider: LanguageSelectionRangeProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.selectionRangeProvider.register(provider); }
export function registerDocumentHighlightProvider(provider: DocumentHighlightProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.documentHighlightProvider.register(provider); }
export function registerMultiDocumentHighlightProvider(provider: MultiDocumentHighlightProvider): IDisposable { return StandaloneServices.get().languageFeaturesService.multiDocumentHighlightProvider.register(provider); }

export function createStandaloneLanguagesApi(): IStandaloneLanguagesApi {
	return Object.freeze({
		register,
		setLanguageConfiguration,
		registerSyntaxProvider,
		registerCompletionProvider,
		registerCodeActionProvider,
		registerCodeLensProvider,
		registerDocumentSymbolProvider,
		registerFormattingProvider,
		registerHoverProvider,
		registerInlayHintsProvider,
		registerInlineCompletionsProvider,
		registerLinkedEditingProvider,
		registerLinkProvider,
		registerParameterHintsProvider,
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
		registerSemanticTokensProvider,
		registerFoldingRangeProvider,
		registerSelectionRangeProvider,
		registerDocumentHighlightProvider,
		registerMultiDocumentHighlightProvider,
	});
}
