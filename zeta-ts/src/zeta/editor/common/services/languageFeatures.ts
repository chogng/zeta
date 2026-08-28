import type { IDisposable } from '../../../base/common/lifecycle.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
import type { LanguageFeatureProviderRegistry } from '../languages/languageFeatureRegistry.js';
import type { LanguageCompletionProvider, LanguageCompletionProviderRegistry } from '../languages/completion/languageCompletionProviders.js';
import type { DocumentHighlightProvider, MultiDocumentHighlightProvider } from '../languages/documentHighlights.js';
import type { SyntaxProviderRegistry } from '../languages/syntax/syntaxProviders.js';
import type { LanguageWorkspaceSymbolProvider } from '../languages/workspaceSymbols.js';
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

/** Provider registries shared by standalone callers and Workbench adapters. */
export interface ILanguageFeaturesService extends IDisposable {
	readonly syntaxProvider: SyntaxProviderRegistry;
	readonly completionProvider: LanguageCompletionProviderRegistry;
	readonly codeActionProvider: LanguageFeatureProviderRegistry<LanguageCodeActionProvider>;
	readonly codeLensProvider: LanguageFeatureProviderRegistry<LanguageCodeLensProvider>;
	readonly documentSymbolProvider: LanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>;
	readonly formattingProvider: LanguageFeatureProviderRegistry<LanguageFormattingProvider>;
	readonly hoverProvider: LanguageFeatureProviderRegistry<LanguageHoverProvider>;
	readonly inlayHintsProvider: LanguageFeatureProviderRegistry<LanguageInlayHintsProvider>;
	readonly inlineCompletionsProvider: LanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>;
	readonly linkedEditingProvider: LanguageFeatureProviderRegistry<LanguageLinkedEditingProvider>;
	readonly linkProvider: LanguageFeatureProviderRegistry<LanguageLinkProvider>;
	readonly parameterHintsProvider: LanguageFeatureProviderRegistry<LanguageParameterHintsProvider>;
	readonly renameProvider: LanguageFeatureProviderRegistry<LanguageRenameProvider>;
	readonly colorProvider: LanguageFeatureProviderRegistry<LanguageColorProvider>;
	readonly definitionProvider: LanguageFeatureProviderRegistry<LanguageDefinitionProvider>;
	readonly declarationProvider: LanguageFeatureProviderRegistry<LanguageDeclarationProvider>;
	readonly implementationProvider: LanguageFeatureProviderRegistry<LanguageImplementationProvider>;
	readonly typeDefinitionProvider: LanguageFeatureProviderRegistry<LanguageTypeDefinitionProvider>;
	readonly referenceProvider: LanguageFeatureProviderRegistry<LanguageReferenceProvider>;
	readonly workspaceSymbolProvider: LanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>;
	readonly callHierarchyProvider: LanguageFeatureProviderRegistry<LanguageCallHierarchyProvider>;
	readonly typeHierarchyProvider: LanguageFeatureProviderRegistry<LanguageTypeHierarchyProvider>;
	readonly semanticTokensProvider: LanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>;
	readonly foldingRangeProvider: LanguageFeatureProviderRegistry<LanguageFoldingRangeProvider>;
	readonly selectionRangeProvider: LanguageFeatureProviderRegistry<LanguageSelectionRangeProvider>;
	readonly documentHighlightProvider: LanguageFeatureProviderRegistry<DocumentHighlightProvider>;
	readonly multiDocumentHighlightProvider: LanguageFeatureProviderRegistry<MultiDocumentHighlightProvider>;
	registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration;
}

export const ILanguageFeaturesService = createServiceIdentifier<ILanguageFeaturesService>('languageFeaturesService');

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
