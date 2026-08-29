import { computeScreenAwareSize, h } from '../../../../base/browser/dom.js';
import { createFastDomNode, type FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { AbstractDisposable } from '../../../../base/common/lifecycle.js';
import { TextEditorCursorStyle } from '../../../common/config/editorOptions.js';
import { TextSelection, TextSelectionSet } from '../../../common/core/selection.js';
import { TextPosition, TextRange } from '../../../common/core/text.js';
import { getTextGraphemeBoundaries } from '../../../common/core/textSegmentation.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type SemanticTokenSource } from '../../../common/services/semanticTokensStyling.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { type EditorLineVisibleRange, type EditorOverlayContext, type EditorRenderingContext, type EditorVisiblePosition } from '../../view/renderingContext.js';

export interface ViewCursorOptions {
	readonly style: TextEditorCursorStyle;
	readonly lineWidth: number;
	readonly lineHeight: number;
}

export const enum CursorPlurality {
	Single,
	MultiPrimary,
	MultiSecondary,
}

interface ViewCursorCharacterPresentation {
	readonly classNames: readonly string[];
	readonly fontStyle?: string;
	readonly fontWeight?: string;
	readonly textDecoration?: string;
}

interface ViewCursorRenderData {
	readonly top: number;
	readonly left: number;
	readonly paddingLeft: number;
	readonly width: number;
	readonly height: number;
	readonly textContent: string;
	readonly presentation?: ViewCursorCharacterPresentation;
}

interface CursorGrapheme {
	readonly position: TextPosition;
	readonly endColumn: number;
	readonly character: string;
}

interface DomCaretGeometry extends EditorVisiblePosition {
	readonly characterRange?: EditorLineVisibleRange;
}

/** Owns one retained caret, its position, rendering data, and DOM writes. */
export class ViewCursor extends AbstractDisposable {
	public readonly domNode: HTMLDivElement;
	private readonly fastDomNode: FastDomNode<HTMLDivElement>;
	private readonly ownerWindow: Window;
	private position = TextPosition.at(0, 0);
	private plurality = CursorPlurality.Single;
	private style: TextEditorCursorStyle;
	private lineWidth: number;
	private readonly lineHeight: number;
	private pauseMovementAnimation = true;
	private renderData: ViewCursorRenderData | undefined;
	private lastRenderedContent = '';
	private lastPauseMovementAnimation: boolean | undefined;

	constructor(
		host: HTMLElement,
		selectionIndex: number,
		options: ViewCursorOptions,
		private readonly model: TextModel,
		private readonly semanticTokenSource: SemanticTokenSource | undefined,
	) {
		super();
		const ownerWindow = host.ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('Editor cursor requires a browser window');
		this.ownerWindow = ownerWindow;
		this.style = options.style;
		this.lineWidth = options.lineWidth;
		this.lineHeight = options.lineHeight;
		this.domNode = h(host.ownerDocument, 'div');
		this.fastDomNode = createFastDomNode(this.domNode);
		this.fastDomNode.setClassName('stanza-editor-caret');
		this.fastDomNode.setAttribute('data-selection-index', String(selectionIndex));
		this.fastDomNode.setAttribute('aria-hidden', 'true');
		this.fastDomNode.setDisplay('none');
		host.append(this.domNode);
	}

	public setPosition(position: TextPosition, plurality: CursorPlurality, pauseMovementAnimation: boolean): void {
		this.position = position;
		this.plurality = plurality;
		this.pauseMovementAnimation = pauseMovementAnimation;
	}

	public setPauseMovementAnimation(pauseMovementAnimation: boolean): void {
		this.pauseMovementAnimation = pauseMovementAnimation;
	}

	public setStyle(style: TextEditorCursorStyle): void {
		this.style = style;
	}

	public setLineWidth(lineWidth: number): void {
		this.lineWidth = lineWidth;
	}

	public prepareRender(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			this.renderData = undefined;
			return;
		}
		const grapheme = this.getGraphemeAwarePosition();
		const caret = this.getCaretGeometry(overlay, grapheme);
		if (!caret) {
			this.renderData = undefined;
			return;
		}
		const characterWidth = caret.characterRange?.width ?? this.getCharacterWidth(overlay, grapheme);
		const characterLeft = caret.characterRange?.left ?? (caret.isRightToLeft ? caret.left - characterWidth : caret.left);
		const width = cursorWidth(this.ownerWindow, this.style, this.lineWidth, characterWidth);
		let left = cursorLeft(this.style, caret.left, characterLeft);
		let paddingLeft = 0;
		if (this.style === TextEditorCursorStyle.Line && width >= 2 && left >= 1) {
			paddingLeft = 1;
			left -= paddingLeft;
		}
		const rowHeight = context.layout.lineHeight;
		const height = cursorHeight(this.style, this.lineHeight, rowHeight);
		const rendersCharacter = this.style === TextEditorCursorStyle.Block || (this.style === TextEditorCursorStyle.Line && width > 2);
		this.renderData = Object.freeze({
			top: context.viewportData.getLineTop(caret.visualLineIndex) + cursorTop(this.style, rowHeight, height),
			left,
			paddingLeft,
			width,
			height,
			textContent: rendersCharacter ? grapheme.character : '',
			presentation: rendersCharacter ? this.getCharacterPresentation(grapheme.position) : undefined,
		});
	}

	public render(): void {
		const renderData = this.renderData;
		if (!renderData) {
			this.fastDomNode.setDisplay('none');
			return;
		}
		this.fastDomNode.setClassName([
			'stanza-editor-caret',
			cursorStyleClass(this.style),
			this.plurality === CursorPlurality.MultiSecondary ? '' : 'primary',
			cursorPluralityClass(this.plurality),
			...(renderData.presentation?.classNames ?? []),
		].filter(Boolean).join(' '));
		if (this.lastRenderedContent !== renderData.textContent) {
			this.lastRenderedContent = renderData.textContent;
			this.domNode.textContent = renderData.textContent;
		}
		this.fastDomNode.setFontStyle(renderData.presentation?.fontStyle ?? '');
		this.fastDomNode.setFontWeight(renderData.presentation?.fontWeight ?? '');
		this.fastDomNode.setTextDecoration(renderData.presentation?.textDecoration ?? '');
		if (this.lastPauseMovementAnimation !== this.pauseMovementAnimation) {
			this.lastPauseMovementAnimation = this.pauseMovementAnimation;
			this.domNode.style.transitionProperty = this.pauseMovementAnimation ? 'none' : '';
		}
		this.fastDomNode.setDisplay('block');
		this.fastDomNode.setTop(renderData.top);
		this.fastDomNode.setLeft(renderData.left);
		this.fastDomNode.setPaddingLeft(renderData.paddingLeft);
		this.fastDomNode.setWidth(renderData.width);
		this.fastDomNode.setHeight(renderData.height);
		this.fastDomNode.setLineHeight(renderData.height);
	}

	protected override disposeCore(): void {
		this.domNode.remove();
	}

	private getGraphemeAwarePosition(): CursorGrapheme {
		const line = this.model.getLineContent(this.position.lineIndex);
		const boundaries = getTextGraphemeBoundaries(line);
		let startColumn = 0;
		for (const boundary of boundaries) {
			if (boundary > this.position.columnIndex) break;
			startColumn = boundary;
		}
		const endColumn = boundaries.find(boundary => boundary > startColumn) ?? startColumn;
		return Object.freeze({
			position: TextPosition.at(this.position.lineIndex, startColumn),
			endColumn,
			character: endColumn === startColumn ? '\u00a0' : line.slice(startColumn, endColumn),
		});
	}

	private getCaretGeometry(context: EditorOverlayContext, grapheme: CursorGrapheme): DomCaretGeometry | undefined {
		const caret = context.visibleRangeForPosition(grapheme.position);
		if (caret) {
			if (grapheme.endColumn === grapheme.position.columnIndex) return caret;
			const range = TextRange.from(grapheme.position, TextPosition.at(grapheme.position.lineIndex, grapheme.endColumn));
			const characterRange = context.linesVisibleRangesForRange(range, false)?.find(candidate => candidate.visualLineIndex === caret.visualLineIndex);
			return characterRange ? Object.freeze({ ...caret, characterRange }) : caret;
		}
		const geometry = createStanzaVisualSelectionGeometry(
			context.model,
			TextSelectionSet.single(TextSelection.collapsedAt(grapheme.position)),
			context.visualLineProjection,
			context.renderLines,
			context.textLeft,
			context.textMeasurer,
		).carets[0];
		return geometry ? Object.freeze({ visualLineIndex: geometry.visualLineIndex, left: geometry.left, isRightToLeft: false }) : undefined;
	}

	private getCharacterWidth(context: EditorOverlayContext, grapheme: CursorGrapheme): number {
		const line = context.model.getLineContent(grapheme.position.lineIndex);
		const visualLineIndex = context.visualLineProjection.visualLineIndexAt(grapheme.position);
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		const startColumn = visualLine?.logicalLineIndex === grapheme.position.lineIndex ? visualLine.startColumn : 0;
		const prefix = line.slice(startColumn, grapheme.position.columnIndex);
		const throughCursor = grapheme.character === '\u00a0' ? `${prefix} ` : `${prefix}${grapheme.character}`;
		return Math.max(1, context.textMeasurer.measureLineWidth(throughCursor) - context.textMeasurer.measureLineWidth(prefix));
	}

	private getCharacterPresentation(position: TextPosition): ViewCursorCharacterPresentation | undefined {
		const token = this.semanticTokenSource?.getLineTokens(position.lineIndex)
			.find(candidate => candidate.startColumn <= position.columnIndex && position.columnIndex < candidate.endColumn);
		if (!token) return undefined;
		const syntaxFontStyle = token.syntaxPresentation?.fontStyle ?? [];
		const decorations = syntaxFontStyle
			.filter(style => style === 'underline' || style === 'strikethrough')
			.map(style => style === 'strikethrough' ? 'line-through' : style);
		return Object.freeze({
			classNames: Object.freeze(['stanza-editor-token', ...(token.presentation ? [token.presentation] : []), ...(token.modifiers ?? [])]),
			...(syntaxFontStyle.includes('italic') ? { fontStyle: 'italic' } : {}),
			...(syntaxFontStyle.includes('bold') ? { fontWeight: 'bold' } : {}),
			...(decorations.length > 0 ? { textDecoration: decorations.join(' ') } : {}),
		});
	}
}

function cursorLeft(style: TextEditorCursorStyle, caretLeft: number, characterLeft: number): number {
	return style === TextEditorCursorStyle.Line || style === TextEditorCursorStyle.LineThin ? caretLeft : characterLeft;
}

function cursorStyleClass(style: TextEditorCursorStyle): string {
	switch (style) {
		case TextEditorCursorStyle.Block: return 'cursor-style-block';
		case TextEditorCursorStyle.Underline: return 'cursor-style-underline';
		case TextEditorCursorStyle.LineThin: return 'cursor-style-line-thin';
		case TextEditorCursorStyle.BlockOutline: return 'cursor-style-block-outline';
		case TextEditorCursorStyle.UnderlineThin: return 'cursor-style-underline-thin';
		default: return 'cursor-style-line';
	}
}

function cursorPluralityClass(plurality: CursorPlurality): string {
	switch (plurality) {
		case CursorPlurality.MultiPrimary: return 'cursor-primary';
		case CursorPlurality.MultiSecondary: return 'cursor-secondary';
		default: return '';
	}
}

function cursorWidth(ownerWindow: Window, style: TextEditorCursorStyle, lineWidth: number, characterWidth: number): number {
	if (style === TextEditorCursorStyle.Line) return computeScreenAwareSize(ownerWindow, lineWidth > 0 ? lineWidth : 2);
	if (style === TextEditorCursorStyle.LineThin) return computeScreenAwareSize(ownerWindow, 1);
	return Math.max(1, characterWidth);
}

function cursorHeight(style: TextEditorCursorStyle, lineHeight: number, rowHeight: number): number {
	if (style === TextEditorCursorStyle.Underline) return 2;
	if (style === TextEditorCursorStyle.UnderlineThin) return 1;
	if (style === TextEditorCursorStyle.Line || style === TextEditorCursorStyle.LineThin) {
		return lineHeight > 0 ? Math.min(lineHeight, rowHeight) : rowHeight;
	}
	return rowHeight;
}

function cursorTop(style: TextEditorCursorStyle, rowHeight: number, height: number): number {
	if (style === TextEditorCursorStyle.Underline || style === TextEditorCursorStyle.UnderlineThin) return rowHeight - height;
	return (rowHeight - height) / 2;
}
