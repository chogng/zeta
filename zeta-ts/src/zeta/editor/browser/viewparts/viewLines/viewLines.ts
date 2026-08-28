import './viewLines.css';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { type EditorVisualLine, type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorLineRange } from '../../../common/viewModel.js';
import { type TextPosition, type TextRange } from '../../../common/core/text.js';
import { type ViewportData } from '../../../common/viewLayout/viewLinesViewportData.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type BracketColorizationSource, type ResolvedSemanticToken, type SemanticTokenSource } from '../semanticTokens/semanticTokenPresentation.js';
import { ViewLine } from './viewLine.js';
import { type ViewLineOptions } from './viewLineOptions.js';
import { ViewLayer } from '../../view/viewLayer.js';
import { type EditorLineVisibleRange, type EditorVisiblePosition } from '../../view/renderingContext.js';

export interface ViewLinesOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
	readonly bracketColorizationSource: BracketColorizationSource | undefined;
	readonly viewLineOptions: ViewLineOptions;
	readonly typicalHalfwidthCharacterWidth: number;
}

/** Projects text and semantic tokens into the generic virtualized ViewLayer. */
export class ViewLines extends Disposable {
	public readonly domNode: HTMLDivElement;
	private readonly model: TextModel;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly bracketColorizationSource: BracketColorizationSource | undefined;
	private readonly layer: ViewLayer<ViewLine>;
	private readonly typicalHalfwidthCharacterWidth: number;

	constructor(options: ViewLinesOptions) {
		super();
		this.model = options.model;
		this.readVisualProjection = options.readVisualProjection;
		this.semanticTokenSource = options.semanticTokenSource;
		this.bracketColorizationSource = options.bracketColorizationSource;
		if (!Number.isFinite(options.typicalHalfwidthCharacterWidth) || options.typicalHalfwidthCharacterWidth <= 0) throw new RangeError('Stanza view-line halfwidth character width must be positive');
		this.typicalHalfwidthCharacterWidth = options.typicalHalfwidthCharacterWidth;
		this.layer = this._register(new ViewLayer<ViewLine>({
			host: options.host,
			readVisualProjection: options.readVisualProjection,
			readProjectionRevision: options.readProjectionRevision,
			lineRenderer: {
				createLine: visualLineIndex => new ViewLine(this.domNode, visualLineIndex, options.viewLineOptions),
				getDomNode: line => line.domNode.domNode,
					renderLine: (line, visualLine) => {
						line.domNode.domNode.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
						line.textElement.style.marginInlineStart = `${visualLine.wrappedTextIndentWidth ?? 0}px`;
						this.projectLineText(line, visualLine, this.resolveSemanticTokensForLine(visualLine));
				},
				layoutLine: (line, lineHeight) => {
					line.layoutLine(lineHeight);
				},
			},
		}));
		this.domNode = this.layer.domNode;
	}

	public get renderedLines(): ReadonlyMap<number, ViewLine> {
		return this.layer.renderedLines;
	}

	public render(viewportData: ViewportData): void {
		this.layer.render(viewportData);
	}

	public linesVisibleRangesForRange(range: TextRange, includeNewLines: boolean): readonly EditorLineVisibleRange[] | undefined {
		this.model.offsetAt(range.start);
		this.model.offsetAt(range.end);
		const projection = this.readVisualProjection();
		if (projection.modelVersion !== this.model.version) return undefined;
		const result: EditorLineVisibleRange[] = [];
		let intersectsRenderedLine = false;
		for (const [visualLineIndex, renderedLine] of this.layer.renderedLines) {
			const visualLine = projection.lineAt(visualLineIndex);
			if (!visualLine || visualLine.logicalLineIndex < range.start.lineIndex || visualLine.logicalLineIndex > range.end.lineIndex) continue;
			const startColumn = visualLine.logicalLineIndex === range.start.lineIndex
				? Math.max(visualLine.startColumn, range.start.columnIndex)
				: visualLine.startColumn;
			const endColumn = visualLine.logicalLineIndex === range.end.lineIndex
				? Math.min(visualLine.endColumn, range.end.columnIndex)
				: visualLine.endColumn;
			const includesNewLine = includeNewLines && visualLine.lastForLogicalLine && visualLine.logicalLineIndex < range.end.lineIndex;
			if (endColumn < startColumn || (endColumn === startColumn && !includesNewLine)) continue;
			intersectsRenderedLine = true;
			const startOffset = startColumn - visualLine.startColumn;
			const endOffset = endColumn - visualLine.startColumn;
			if (!renderedLine.hasTextOffset(startOffset) || !renderedLine.hasTextOffset(endOffset)) return undefined;
			const ranges = renderedLine.getHorizontalRanges(startOffset, endOffset);
			if (!ranges) return undefined;
			const lineRanges = ranges.map(horizontalRange => ({
				visualLineIndex,
				left: horizontalRange.left,
				width: horizontalRange.width,
			}));
			if (includesNewLine) {
				const lastRange = lineRanges[lineRanges.length - 1];
				if (!lastRange) return undefined;
				lastRange.width += this.typicalHalfwidthCharacterWidth;
				if (renderedLine.isRightToLeft()) lastRange.left -= this.typicalHalfwidthCharacterWidth;
			}
			result.push(...lineRanges.map(lineRange => Object.freeze(lineRange)));
		}
		return intersectsRenderedLine ? Object.freeze(result) : undefined;
	}

	public visibleRangeForPosition(position: TextPosition): EditorVisiblePosition | undefined {
		this.model.offsetAt(position);
		const projection = this.readVisualProjection();
		if (projection.modelVersion !== this.model.version) return undefined;
		const visualLineIndex = projection.visualLineIndexAt(position);
		const visualLine = projection.lineAt(visualLineIndex);
		const renderedLine = this.layer.renderedLines.get(visualLineIndex);
		if (!visualLine || !renderedLine) return undefined;
		const offset = position.columnIndex - visualLine.startColumn;
		if (!renderedLine.hasTextOffset(offset)) return undefined;
		const left = renderedLine.getCaretLeft(offset);
		return left === undefined ? undefined : Object.freeze({ visualLineIndex, left });
	}

	/** Reprojects semantic tokens without rebuilding the visible row window. */
	public renderVisibleLineText(): void {
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

	private projectLineText(line: ViewLine, visualLine: { readonly logicalLineIndex: number; readonly startColumn: number; readonly endColumn: number }, tokens: readonly ResolvedSemanticToken[]): void {
		const fullText = this.model.getLineContent(visualLine.logicalLineIndex);
		const text = fullText.slice(visualLine.startColumn, visualLine.endColumn);
		const brackets = this.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
		line.renderText(
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
			...(token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
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
