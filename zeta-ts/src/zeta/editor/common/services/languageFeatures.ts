import type { IDisposable } from '../../../base/common/lifecycle.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
import type { LanguageFeatureProviderMetadata, OwnedLanguageFeatureProviderRegistry } from '../ownedLanguageFeatureProviderRegistry.js';
import type { LanguageCompletionProvider, LanguageCompletionProviderRegistry } from '../languages/completion/languageCompletionProviders.js';
import type { DocumentHighlightProvider, MultiDocumentHighlightProvider } from '../languages.js';
import type { SyntaxProviderRegistry } from '../languages/syntax/syntaxProviders.js';
import type { LanguageWorkspaceSymbolProvider } from '../languages/workspaceSymbols.js';
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

/** Provider registries shared by standalone callers and Workbench adapters. */
export interface IEditorLanguageFeaturesService extends IDisposable {
	readonly syntaxProvider: SyntaxProviderRegistry;
	readonly completionProvider: LanguageCompletionProviderRegistry;
	readonly codeActionProvider: OwnedLanguageFeatureProviderRegistry<LanguageCodeActionProvider>;
	readonly codeLensProvider: OwnedLanguageFeatureProviderRegistry<LanguageCodeLensProvider>;
	readonly documentSymbolProvider: OwnedLanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>;
	readonly formattingProvider: OwnedLanguageFeatureProviderRegistry<LanguageFormattingProvider>;
	readonly hoverProvider: OwnedLanguageFeatureProviderRegistry<LanguageHoverProvider>;
	readonly inlayHintsProvider: OwnedLanguageFeatureProviderRegistry<LanguageInlayHintsProvider>;
	readonly inlineCompletionsProvider: OwnedLanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>;
	readonly linkedEditingProvider: OwnedLanguageFeatureProviderRegistry<LanguageLinkedEditingProvider>;
	readonly linkProvider: OwnedLanguageFeatureProviderRegistry<LanguageLinkProvider>;
	readonly parameterHintsProvider: OwnedLanguageFeatureProviderRegistry<LanguageParameterHintsProvider>;
	readonly renameProvider: OwnedLanguageFeatureProviderRegistry<LanguageRenameProvider>;
	readonly colorProvider: OwnedLanguageFeatureProviderRegistry<LanguageColorProvider>;
	readonly definitionProvider: OwnedLanguageFeatureProviderRegistry<LanguageDefinitionProvider>;
	readonly declarationProvider: OwnedLanguageFeatureProviderRegistry<LanguageDeclarationProvider>;
	readonly implementationProvider: OwnedLanguageFeatureProviderRegistry<LanguageImplementationProvider>;
	readonly typeDefinitionProvider: OwnedLanguageFeatureProviderRegistry<LanguageTypeDefinitionProvider>;
	readonly referenceProvider: OwnedLanguageFeatureProviderRegistry<LanguageReferenceProvider>;
	readonly workspaceSymbolProvider: OwnedLanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>;
	readonly callHierarchyProvider: OwnedLanguageFeatureProviderRegistry<LanguageCallHierarchyProvider>;
	readonly typeHierarchyProvider: OwnedLanguageFeatureProviderRegistry<LanguageTypeHierarchyProvider>;
	readonly semanticTokensProvider: OwnedLanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>;
	readonly foldingRangeProvider: OwnedLanguageFeatureProviderRegistry<LanguageFoldingRangeProvider>;
	readonly selectionRangeProvider: OwnedLanguageFeatureProviderRegistry<LanguageSelectionRangeProvider>;
	readonly documentHighlightProvider: OwnedLanguageFeatureProviderRegistry<DocumentHighlightProvider & LanguageFeatureProviderMetadata>;
	readonly multiDocumentHighlightProvider: OwnedLanguageFeatureProviderRegistry<MultiDocumentHighlightProvider & LanguageFeatureProviderMetadata>;
	registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration;
}

export const IEditorLanguageFeaturesService = createServiceIdentifier<IEditorLanguageFeaturesService>('languageFeaturesService');

/** One runtime generation contributing several provider kinds atomically. */
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
