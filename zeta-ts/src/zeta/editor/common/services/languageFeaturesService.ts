import { runWithBufferedEvents } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { OwnedLanguageFeatureProviderRegistry, type LanguageFeatureProviderMetadata, type LanguageFeatureProviderRegistration } from '../ownedLanguageFeatureProviderRegistry.js';
import { LanguageCompletionProviderRegistry, type LanguageCompletionProviderRegistration } from '../languages/completion/languageCompletionProviders.js';
import { createLanguageWordCompletionProvider } from '../languages/completion/languageWordCompletionProvider.js';
import type { DocumentHighlightProvider, MultiDocumentHighlightProvider } from '../languages.js';
import { createLanguageLexicalSyntaxProvider } from '../languages/languageLexicalSyntaxProvider.js';
import type { LanguageConfigurationSource } from '../languages/ownedLanguageConfigurationContributions.js';
import { SyntaxProviderRegistry } from '../languages/syntax/syntaxProviders.js';
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
import type { IEditorLanguageFeaturesService, LanguageProviderBatch, LanguageProviderBatchRegistration } from './languageFeatures.js';

/** Owns the Editor provider registries without knowing their Workbench implementations. */
export class EditorLanguageFeaturesService extends Disposable implements IEditorLanguageFeaturesService {
	public readonly syntaxProvider: SyntaxProviderRegistry;
	public readonly completionProvider: LanguageCompletionProviderRegistry;
	public readonly codeActionProvider: OwnedLanguageFeatureProviderRegistry<LanguageCodeActionProvider>;
	public readonly codeLensProvider: OwnedLanguageFeatureProviderRegistry<LanguageCodeLensProvider>;
	public readonly documentSymbolProvider: OwnedLanguageFeatureProviderRegistry<LanguageDocumentSymbolProvider>;
	public readonly formattingProvider: OwnedLanguageFeatureProviderRegistry<LanguageFormattingProvider>;
	public readonly hoverProvider: OwnedLanguageFeatureProviderRegistry<LanguageHoverProvider>;
	public readonly inlayHintsProvider: OwnedLanguageFeatureProviderRegistry<LanguageInlayHintsProvider>;
	public readonly inlineCompletionsProvider: OwnedLanguageFeatureProviderRegistry<LanguageInlineCompletionsProvider>;
	public readonly linkedEditingProvider: OwnedLanguageFeatureProviderRegistry<LanguageLinkedEditingProvider>;
	public readonly linkProvider: OwnedLanguageFeatureProviderRegistry<LanguageLinkProvider>;
	public readonly parameterHintsProvider: OwnedLanguageFeatureProviderRegistry<LanguageParameterHintsProvider>;
	public readonly renameProvider: OwnedLanguageFeatureProviderRegistry<LanguageRenameProvider>;
	public readonly colorProvider: OwnedLanguageFeatureProviderRegistry<LanguageColorProvider>;
	public readonly definitionProvider: OwnedLanguageFeatureProviderRegistry<LanguageDefinitionProvider>;
	public readonly declarationProvider: OwnedLanguageFeatureProviderRegistry<LanguageDeclarationProvider>;
	public readonly implementationProvider: OwnedLanguageFeatureProviderRegistry<LanguageImplementationProvider>;
	public readonly typeDefinitionProvider: OwnedLanguageFeatureProviderRegistry<LanguageTypeDefinitionProvider>;
	public readonly referenceProvider: OwnedLanguageFeatureProviderRegistry<LanguageReferenceProvider>;
	public readonly workspaceSymbolProvider: OwnedLanguageFeatureProviderRegistry<LanguageWorkspaceSymbolProvider>;
	public readonly callHierarchyProvider: OwnedLanguageFeatureProviderRegistry<LanguageCallHierarchyProvider>;
	public readonly typeHierarchyProvider: OwnedLanguageFeatureProviderRegistry<LanguageTypeHierarchyProvider>;
	public readonly semanticTokensProvider: OwnedLanguageFeatureProviderRegistry<LanguageSemanticTokensProvider>;
	public readonly foldingRangeProvider: OwnedLanguageFeatureProviderRegistry<LanguageFoldingRangeProvider>;
	public readonly selectionRangeProvider: OwnedLanguageFeatureProviderRegistry<LanguageSelectionRangeProvider>;
	public readonly documentHighlightProvider: OwnedLanguageFeatureProviderRegistry<DocumentHighlightProvider & LanguageFeatureProviderMetadata>;
	public readonly multiDocumentHighlightProvider: OwnedLanguageFeatureProviderRegistry<MultiDocumentHighlightProvider & LanguageFeatureProviderMetadata>;

	constructor(languageConfigurations: LanguageConfigurationSource) {
		super();
		this.syntaxProvider = this._register(new SyntaxProviderRegistry());
		this._register(this.syntaxProvider.register(createLanguageLexicalSyntaxProvider({ languageConfigurations })));
		this.completionProvider = this._register(new LanguageCompletionProviderRegistry());
		this._register(this.completionProvider.register(createLanguageWordCompletionProvider()));
		this.codeActionProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.codeLensProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.documentSymbolProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.formattingProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.hoverProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.inlayHintsProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.inlineCompletionsProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.linkedEditingProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.linkProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.parameterHintsProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.renameProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.colorProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.definitionProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.declarationProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.implementationProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.typeDefinitionProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.referenceProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.workspaceSymbolProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.callHierarchyProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.typeHierarchyProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.semanticTokensProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.foldingRangeProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.selectionRangeProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.documentHighlightProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
		this.multiDocumentHighlightProvider = this._register(new OwnedLanguageFeatureProviderRegistry());
	}

	public registerProviderBatch(providers: LanguageProviderBatch): LanguageProviderBatchRegistration {
		const completions = this.completionProvider.registerGroup([]);
		const hovers = this.hoverProvider.registerGroup([]);
		const formatting = this.formattingProvider.registerGroup([]);
		const inlayHints = this.inlayHintsProvider.registerGroup([]);
		const linkedEditing = this.linkedEditingProvider.registerGroup([]);
		const parameterHints = this.parameterHintsProvider.registerGroup([]);
		const registrations = { completions, hovers, formatting, inlayHints, linkedEditing, parameterHints };
		let current = emptyProviderBatch();
		let disposed = false;
		const replace = (replacement: LanguageProviderBatch): void => {
			if (disposed) {
				throw new ReferenceError('Language provider batch registration is already disposed');
			}
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
			if (disposed) {
				return;
			}
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
	if (!value || typeof value !== 'object' || Array.isArray(value)) {
		throw new TypeError('Language provider batch must be an object');
	}
	const record = value as Record<string, unknown>;
	const supported = new Set(['completions', 'hovers', 'formatting', 'inlayHints', 'linkedEditing', 'parameterHints']);
	if (Object.keys(record).some(key => !supported.has(key))) {
		throw new TypeError('Language provider batch contains an unsupported provider kind');
	}
	return Object.freeze({
		completions: frozenProviderArray(value.completions, 'completion'),
		hovers: frozenProviderArray(value.hovers, 'hover'),
		formatting: frozenProviderArray(value.formatting, 'formatting'),
		inlayHints: frozenProviderArray(value.inlayHints, 'Inlay Hints'),
		linkedEditing: frozenProviderArray(value.linkedEditing, 'Linked Editing'),
		parameterHints: frozenProviderArray(value.parameterHints, 'Parameter Hints'),
	});
}

function emptyProviderBatch(): Required<LanguageProviderBatch> {
	const empty = Object.freeze([]);
	return Object.freeze({ completions: empty, hovers: empty, formatting: empty, inlayHints: empty, linkedEditing: empty, parameterHints: empty });
}

function frozenProviderArray<T>(value: readonly T[] | undefined, owner: string): readonly T[] {
	if (value === undefined) {
		return Object.freeze([]);
	}
	if (!Array.isArray(value)) {
		throw new TypeError(`Language ${owner} providers must be an array`);
	}
	return Object.freeze([...value]);
}
