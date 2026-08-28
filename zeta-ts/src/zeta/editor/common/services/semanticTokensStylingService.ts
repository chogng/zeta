import { AbstractDisposable, combinedDisposable } from '../../../base/common/lifecycle.js';
import type { LanguageToken } from '../tokens/languageTokens.js';
import { SemanticTokensProviderStyling } from './semanticTokensProviderStyling.js';
import { SemanticTokenModifier, SemanticTokenPresentation, type ISemanticTokensStylingService, type ResolvedSemanticToken, type SemanticTokenLine, type SemanticTokenModelSource, type SemanticTokenSource, type SemanticTokenStylingResolver } from './semanticTokensStyling.js';

/** Adapts common token indexes into immutable styled line sources for one editor lifetime. */
export class SemanticTokensStylingService extends AbstractDisposable implements ISemanticTokensStylingService {
	private readonly defaultStyling = new SemanticTokensProviderStyling();

	public createSource(source: SemanticTokenModelSource, styling: SemanticTokenStylingResolver = this.defaultStyling): SemanticTokenSource {
		this.assertNotDisposed();
		if (!source || typeof source.getLineTokens !== 'function' || typeof source.onDidChange !== 'function') throw new TypeError('Semantic token styling requires a token model source');
		if (!styling || typeof styling.resolve !== 'function') throw new TypeError('Semantic token styling requires a resolver');
		const service = this;
		const onDidChange: SemanticTokenSource['onDidChange'] = listener => {
			service.assertNotDisposed();
			return source.onDidChange(() => listener());
		};
		return Object.freeze({
			textModel: source.textModel,
			onDidChange,
			get lines(): readonly SemanticTokenLine[] {
				service.assertNotDisposed();
				return Object.freeze(source.lines.map(line => Object.freeze({ lineIndex: line.lineIndex, tokens: resolveLineTokens(line.tokens, styling) })));
			},
			getLineTokens: (lineIndex: number) => {
				service.assertNotDisposed();
				return resolveLineTokens(source.getLineTokens(lineIndex), styling);
			},
		});
	}

	public createOverlay(base: SemanticTokenSource, overlay: SemanticTokenSource): SemanticTokenSource {
		this.assertNotDisposed();
		if (base.textModel !== overlay.textModel) throw new TypeError('Semantic-token overlay sources must share one text model');
		const service = this;
		const onDidChange: SemanticTokenSource['onDidChange'] = listener => {
			service.assertNotDisposed();
			return combinedDisposable(base.onDidChange(listener), overlay.onDidChange(listener));
		};
		const getLineTokens = (lineIndex: number): readonly ResolvedSemanticToken[] => {
			service.assertNotDisposed();
			return mergeResolvedLineTokens(base.getLineTokens(lineIndex), overlay.getLineTokens(lineIndex));
		};
		return Object.freeze({
			textModel: base.textModel,
			onDidChange,
			get lines(): readonly SemanticTokenLine[] {
				service.assertNotDisposed();
				const lineIndexes = new Set([...base.lines.map(line => line.lineIndex), ...overlay.lines.map(line => line.lineIndex)]);
				return Object.freeze([...lineIndexes].sort((left, right) => left - right).map(lineIndex => Object.freeze({ lineIndex, tokens: getLineTokens(lineIndex) })));
			},
			getLineTokens,
		});
	}

	protected disposeCore(): void {}
}

function resolveLineTokens(tokens: readonly LanguageToken[], styling: SemanticTokenStylingResolver): readonly ResolvedSemanticToken[] {
	const resolved: ResolvedSemanticToken[] = [];
	for (const token of tokens) {
		const tokenStyling = styling.resolve(token);
		if (!tokenStyling || typeof tokenStyling !== 'object' || !Array.isArray(tokenStyling.modifiers)) throw new TypeError('Semantic token resolver returned invalid styling');
		if (tokenStyling.presentation !== undefined && !Object.values(SemanticTokenPresentation).includes(tokenStyling.presentation)) throw new TypeError(`Unknown semantic token presentation '${tokenStyling.presentation}'`);
		if (new Set(tokenStyling.modifiers).size !== tokenStyling.modifiers.length || tokenStyling.modifiers.some(modifier => !Object.values(SemanticTokenModifier).includes(modifier))) throw new TypeError('Unknown or duplicate semantic token modifier');
		if (tokenStyling.presentation === undefined && token.presentation === undefined) continue;
		resolved.push(Object.freeze({
			startColumn: token.range.start.columnIndex,
			endColumn: token.range.end.columnIndex,
			...(tokenStyling.presentation === undefined ? {} : { presentation: tokenStyling.presentation }),
			...(tokenStyling.modifiers.length === 0 ? {} : { modifiers: tokenStyling.modifiers }),
			...(token.presentation === undefined ? {} : { syntaxPresentation: token.presentation }),
		}));
	}
	return Object.freeze(resolved);
}

function mergeResolvedLineTokens(base: readonly ResolvedSemanticToken[], overlay: readonly ResolvedSemanticToken[]): readonly ResolvedSemanticToken[] {
	if (overlay.length === 0) return base;
	if (base.length === 0) return overlay;
	const boundaries = [...new Set([...base.flatMap(token => [token.startColumn, token.endColumn]), ...overlay.flatMap(token => [token.startColumn, token.endColumn])])].sort((left, right) => left - right);
	const result: ResolvedSemanticToken[] = [];
	for (let index = 0; index + 1 < boundaries.length; index += 1) {
		const startColumn = boundaries[index]!;
		const endColumn = boundaries[index + 1]!;
		const semantic = overlay.find(token => token.startColumn <= startColumn && token.endColumn >= endColumn);
		const lexical = base.find(token => token.startColumn <= startColumn && token.endColumn >= endColumn);
		const token = semantic ?? lexical;
		if (!token) continue;
		result.push(Object.freeze({
			startColumn,
			endColumn,
			...(token.presentation === undefined ? {} : { presentation: token.presentation }),
			...(token.modifiers === undefined ? {} : { modifiers: token.modifiers }),
			...(semantic || token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
		}));
	}
	return Object.freeze(result);
}
