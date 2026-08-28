import type { LanguageToken } from '../tokens/languageTokens.js';
import { SemanticTokenModifier, SemanticTokenPresentation, type SemanticTokenStyling, type SemanticTokenStylingResolver } from './semanticTokensStyling.js';

export type SemanticTokenPresentationResolver = (token: LanguageToken) => SemanticTokenPresentation | undefined;

/** Resolves provider token names into the editor's closed styling vocabulary. */
export class SemanticTokensProviderStyling implements SemanticTokenStylingResolver {
	constructor(private readonly resolvePresentation: SemanticTokenPresentationResolver = resolveSemanticTokenPresentation) {
		if (typeof resolvePresentation !== 'function') throw new TypeError('Semantic token presentation resolver must be a function');
	}

	public resolve(token: LanguageToken): SemanticTokenStyling {
		const presentation = this.resolvePresentation(token);
		if (presentation !== undefined && !Object.values(SemanticTokenPresentation).includes(presentation)) throw new TypeError(`Unknown semantic token presentation '${presentation}'`);
		const modifiers = resolveSemanticTokenModifiers(token);
		return Object.freeze({ ...(presentation === undefined ? {} : { presentation }), modifiers });
	}
}

export function resolveSemanticTokenPresentation(token: LanguageToken): SemanticTokenPresentation | undefined {
	switch (token.tokenType) {
		case 'comment': return SemanticTokenPresentation.Comment;
		case 'keyword':
		case 'modifier': return SemanticTokenPresentation.Keyword;
		case 'string': return SemanticTokenPresentation.String;
		case 'number': return SemanticTokenPresentation.Number;
		case 'regexp': return SemanticTokenPresentation.Regexp;
		case 'class':
		case 'enum':
		case 'interface':
		case 'namespace':
		case 'struct':
		case 'type':
		case 'typeParameter': return SemanticTokenPresentation.Type;
		case 'function':
		case 'method': return SemanticTokenPresentation.Function;
		case 'enumMember':
		case 'event':
		case 'parameter':
		case 'property':
		case 'variable': return SemanticTokenPresentation.Variable;
		case 'operator': return SemanticTokenPresentation.Operator;
		default: return undefined;
	}
}

export function resolveSemanticTokenModifiers(token: LanguageToken): readonly SemanticTokenModifier[] {
	const resolved = new Set<SemanticTokenModifier>();
	for (const modifier of token.modifiers) {
		switch (modifier) {
			case 'declaration':
			case 'definition': resolved.add(SemanticTokenModifier.Declaration); break;
			case 'readonly': resolved.add(SemanticTokenModifier.Readonly); break;
			case 'static': resolved.add(SemanticTokenModifier.Static); break;
			case 'deprecated': resolved.add(SemanticTokenModifier.Deprecated); break;
			case 'abstract': resolved.add(SemanticTokenModifier.Abstract); break;
			case 'async': resolved.add(SemanticTokenModifier.Async); break;
		}
	}
	return Object.freeze([...resolved]);
}
