import './viewLines.css';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { type EditorVisualLine, type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorLineRange } from '../../../common/viewModel.js';
import { type ViewportData } from '../../../common/viewLayout/viewLinesViewportData.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type BracketColorizationSource, type ResolvedSemanticToken, type SemanticTokenSource, projectStanzaSemanticTokenLine } from '../semanticTokens/semanticTokenPresentation.js';
import { createStanzaRenderedLine, type RenderedLine } from './renderedLine.js';
import { ViewLayer } from '../../view/viewLayer.js';

export type ViewLinesTextDirection = 'auto' | 'ltr' | 'rtl';

export interface ViewLinesPartOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly textDirection: ViewLinesTextDirection;
}

/** Projects text and semantic tokens into the generic virtualized ViewLayer. */
export class ViewLinesPart extends Disposable {
	readonly domNode: HTMLDivElement;
	private readonly model: TextModel;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly bracketColorizationSource: BracketColorizationSource | undefined;
	private readonly textDirection: ViewLinesTextDirection;
	private readonly layer: ViewLayer<RenderedLine>;

	constructor(options: ViewLinesPartOptions) {
		super();
		this.model = options.model;
		this.readVisualProjection = options.readVisualProjection;
		this.semanticTokenSource = options.semanticTokenSource;
		this.bracketColorizationSource = options.bracketColorizationSource;
		this.textDirection = options.textDirection;
		this.layer = this._register(new ViewLayer<RenderedLine>({
			host: options.host,
			readVisualProjection: options.readVisualProjection,
			readProjectionRevision: options.readProjectionRevision,
			lineRenderer: {
				createLine: visualLineIndex => createStanzaRenderedLine(this.domNode.ownerDocument, visualLineIndex),
				getDomNode: line => line.domNode.domNode,
					renderLine: (line, visualLine) => {
						line.domNode.domNode.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
						line.textElement.dir = this.textDirection;
						line.textElement.style.marginInlineStart = `${visualLine.wrappedTextIndentWidth ?? 0}px`;
						this.projectLineText(line, visualLine, this.resolveSemanticTokensForLine(visualLine));
				},
				layoutLine: (line, lineHeight) => {
					line.domNode.setHeight(lineHeight);
					line.domNode.setLineHeight(lineHeight);
				},
			},
		}));
		this.domNode = this.layer.domNode;
	}

	get renderedLines(): ReadonlyMap<number, RenderedLine> {
		return this.layer.renderedLines;
	}

	render(viewportData: ViewportData): void {
		this.layer.render(viewportData);
	}

	/** Reprojects semantic tokens without rebuilding the visible row window. */
	renderVisibleLineText(): void {
		const semanticTokens = this.resolveSemanticTokenRange(this.layer.renderedLineRange);
		const visualProjection = this.readVisualProjection();
		for (const [visualLineIndex, line] of this.layer.renderedLines) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (visualLine) this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
		}
	}

	private resolveSemanticTokensForLine(visualLine: EditorVisualLine): readonly ResolvedSemanticToken[] {
		return this.semanticTokenSource?.getLineTokens(visualLine.logicalLineIndex) ?? [];
	}

	private projectLineText(line: RenderedLine, visualLine: { readonly logicalLineIndex: number; readonly startColumn: number; readonly endColumn: number }, tokens: readonly ResolvedSemanticToken[]): void {
		const fullText = this.model.getLineContent(visualLine.logicalLineIndex);
		const text = fullText.slice(visualLine.startColumn, visualLine.endColumn);
		const brackets = this.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
		projectStanzaSemanticTokenLine(
			line.textElement,
			text,
			clipSemanticTokens(tokens, visualLine.startColumn, visualLine.endColumn),
			clipBracketColorizations(brackets, visualLine.startColumn, visualLine.endColumn),
		);
	}

	private resolveSemanticTokenRange(range: EditorLineRange): ReadonlyMap<number, readonly ResolvedSemanticToken[]> {
		const source = this.semanticTokenSource;
		if (!source) return new Map();
		const tokens = new Map<number, readonly ResolvedSemanticToken[]>();
		const projection = this.readVisualProjection();
		for (let visualLineIndex = range.startLineIndex; visualLineIndex < range.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = projection.lineAt(visualLineIndex);
			if (visualLine && !tokens.has(visualLine.logicalLineIndex)) tokens.set(visualLine.logicalLineIndex, source.getLineTokens(visualLine.logicalLineIndex));
		}
		return tokens;
	}
}

function clipSemanticTokens(tokens: readonly ResolvedSemanticToken[], startColumn: number, endColumn: number): readonly ResolvedSemanticToken[] {
	return Object.freeze(tokens.flatMap(token => {
		const start = Math.max(token.startColumn, startColumn);
		const end = Math.min(token.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({
			startColumn: start - startColumn,
			endColumn: end - startColumn,
			presentation: token.presentation,
			...(token.modifiers && token.modifiers.length > 0 ? { modifiers: token.modifiers } : {}),
		})];
	}));
}
function clipBracketColorizations(brackets: readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[], startColumn: number, endColumn: number): readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[] {
	return Object.freeze(brackets.flatMap(bracket => {
		const start = Math.max(bracket.startColumn, startColumn);
		const end = Math.min(bracket.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({ startColumn: start - startColumn, endColumn: end - startColumn, level: bracket.level })];
	}));
}
