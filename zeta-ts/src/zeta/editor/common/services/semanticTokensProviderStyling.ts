import { type LanguageToken } from '../tokens/languageTokens.js';
import { LanguageTokenStylingResolver } from './languageTokenStylingResolver.js';
import { type SemanticTokenStyling, type SemanticTokenStylingResolver } from './resolvedSemanticTokens.js';
import { type DocumentTokensProvider } from './semanticTokensStyling.js';

/** Resolves the object-token vocabulary for one semantic-token provider. */
export class SemanticTokensProviderStyling implements SemanticTokenStylingResolver {
	private readonly resolver = new LanguageTokenStylingResolver();

	constructor(readonly provider: DocumentTokensProvider) {
		if (!provider || typeof provider.provideSemanticTokens !== 'function') throw new TypeError('Semantic token styling requires a document token provider');
	}

	public resolve(token: LanguageToken): SemanticTokenStyling {
		return this.resolver.resolve(token);
	}
}
