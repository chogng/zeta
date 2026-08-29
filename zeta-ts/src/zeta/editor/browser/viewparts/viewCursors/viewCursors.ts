import "./viewCursors.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { TextEditorCursorBlinkingStyle, TextEditorCursorStyle } from '../../../common/config/editorOptions.js';
import { EditorSelectionChangeReason, type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from '../../../common/core/selection.js';
import { TextPosition, TextRange } from '../../../common/core/text.js';
import { getTextGraphemeBoundaries } from '../../../common/core/textSegmentation.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../../../common/model/trackedRange.js';
import { type SemanticTokenSource } from '../../../common/services/semanticTokensStyling.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorLineVisibleRange, type EditorOverlayContext, type EditorVisiblePosition } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';
import { ViewCursor, type ViewCursorCharacterPresentation, type ViewCursorOptions, type ViewCursorPlurality } from './viewCursor.js';

export interface ViewCursorsOptions extends ViewCursorOptions {
	readonly host: HTMLElement;
	readonly blinking: TextEditorCursorBlinkingStyle;
	readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	readonly semanticTokenSource?: SemanticTokenSource;
}

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursors extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly model: TextModel;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly semanticTokenSource: SemanticTokenSource | undefined;
	private readonly compositionRows: ViewPartRows;
	private readonly smoothCaretAnimation: 'off' | 'explicit' | 'on';
	private cursorOptions: ViewCursorOptions;
	private readonly cursors = new Map<number, ViewCursor>();
	private compositionRange: TrackedRange | undefined;
	private previousSelectionCount: number;
	private pauseMovementAnimation = true;
	private movementRenderGeneration = 0;

	constructor(context: EditorViewContext, options: ViewCursorsOptions, model: TextModel, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.domNode = h(options.host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-cursors-layer';
		this.domNode.classList.add(cursorBlinkingClass(options.blinking));
		this.domNode.classList.toggle('cursor-smooth-caret-animation', options.smoothCaretAnimation !== 'off');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this.compositionRows = this._register(new ViewPartRows(this.domNode, 'stanza-editor-composition-layer', 'stanza-editor-composition-row'));
		this.domNode.append(this.compositionRows.domNode);
		this.model = model;
		this.selectionController = selectionController;
		this.semanticTokenSource = options.semanticTokenSource;
		this.smoothCaretAnimation = options.smoothCaretAnimation;
		this.cursorOptions = options;
		this.previousSelectionCount = selectionController?.selections.selections.length ?? 0;
		this._register(toDisposable(() => this.compositionRange?.dispose()));
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public setCompositionRange(range: TextRange | undefined): void {
		const next = range ? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges) : undefined;
		this.compositionRange?.dispose();
		this.compositionRange = next;
		this.renderNow(this.context.renderingContext);
	}

	public setStyle(style: TextEditorCursorStyle): void {
		if (style === this.cursorOptions.style) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, style });
		for (const cursor of this.cursors.values()) cursor.setStyle(style);
		this.renderNow(this.context.renderingContext);
	}

	public setLineWidth(lineWidth: number): void {
		if (lineWidth === this.cursorOptions.lineWidth) return;
		this.cursorOptions = Object.freeze({ ...this.cursorOptions, lineWidth });
		for (const cursor of this.cursors.values()) cursor.setLineWidth(lineWidth);
		this.renderNow(this.context.renderingContext);
	}

	public render(context: EditorRenderingContext): void {
		this.renderCursors(context, this.pauseMovementAnimation);
	}

	public renderSelection(context: EditorRenderingContext, reason: EditorSelectionChangeReason): void {
		const selectionCount = this.selectionController?.selections.selections.length ?? 0;
		this.pauseMovementAnimation = !this.shouldAnimateMovement(reason, selectionCount);
		this.previousSelectionCount = selectionCount;
		this.renderCursors(context, this.pauseMovementAnimation);
		for (const animation of this.domNode.getAnimations?.() ?? []) animation.currentTime = 0;
		const generation = ++this.movementRenderGeneration;
		queueMicrotask(() => {
			if (generation === this.movementRenderGeneration) this.pauseMovementAnimation = true;
		});
	}

	public renderTokens(context: EditorRenderingContext): void {
		this.movementRenderGeneration += 1;
		this.pauseMovementAnimation = true;
		this.renderCursors(context, true);
	}

	private renderCursors(context: EditorRenderingContext, pauseMovementAnimation: boolean): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const rows = this.compositionRows.render(context);
		for (const row of rows.values()) reset(row);
		projectStanzaCompositionOverlay(overlay, this.compositionRange?.range, rows);
		const renderedCursorIndexes = projectStanzaCursorOverlays({
			renderingContext: context,
			overlay,
			controller: this.selectionController,
			host: this.domNode,
			cursors: this.cursors,
			cursorOptions: this.cursorOptions,
			lineHeight: context.layout.lineHeight,
			pauseMovementAnimation,
			semanticTokenSource: this.semanticTokenSource,
		});
		for (const [selectionIndex, cursor] of this.cursors) {
			if (renderedCursorIndexes.has(selectionIndex)) {
				continue;
			}
			cursor.domNode.remove();
			this.cursors.delete(selectionIndex);
		}
	}

	private shouldAnimateMovement(reason: EditorSelectionChangeReason, selectionCount: number): boolean {
		if (this.smoothCaretAnimation === 'off' || selectionCount !== this.previousSelectionCount) return false;
		if (this.smoothCaretAnimation === 'on') return true;
		return reason === EditorSelectionChangeReason.Explicit || reason === EditorSelectionChangeReason.CursorOperation || reason === EditorSelectionChangeReason.CursorUndo;
	}
}

function cursorBlinkingClass(blinking: TextEditorCursorBlinkingStyle): string {
	switch (blinking) {
		case TextEditorCursorBlinkingStyle.Smooth: return 'cursor-blinking-smooth';
		case TextEditorCursorBlinkingStyle.Phase: return 'cursor-blinking-phase';
		case TextEditorCursorBlinkingStyle.Expand: return 'cursor-blinking-expand';
		case TextEditorCursorBlinkingStyle.Solid: return 'cursor-blinking-solid';
		case TextEditorCursorBlinkingStyle.Hidden: return 'cursor-blinking-hidden';
		default: return 'cursor-blinking-blink';
	}
}

interface CursorOverlayProjectionOptions {
	readonly renderingContext: EditorRenderingContext;
	readonly overlay: EditorOverlayContext;
	readonly controller: EditorSelectionController | undefined;
	readonly host: HTMLElement;
	readonly cursors: Map<number, ViewCursor>;
	readonly cursorOptions: ViewCursorOptions;
	readonly lineHeight: number;
	readonly pauseMovementAnimation: boolean;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
}

function projectStanzaCursorOverlays(options: CursorOverlayProjectionOptions): ReadonlySet<number> {
	const {
		renderingContext,
		overlay,
		controller,
		host,
		cursors,
		cursorOptions,
		lineHeight,
		pauseMovementAnimation,
		semanticTokenSource,
	} = options;
	const renderedCursorIndexes = new Set<number>();
	if (!controller) return renderedCursorIndexes;
	const cursorGraphemes = controller.selections.selections.map(selection => cursorGrapheme(overlay, selection.active));
	const domCarets = new Map<number, DomCaretGeometry>();
	for (let selectionIndex = 0; selectionIndex < cursorGraphemes.length; selectionIndex += 1) {
		const geometry = domCaretGeometry(overlay, cursorGraphemes[selectionIndex]!.position);
		if (geometry) domCarets.set(selectionIndex, geometry);
	}
	const cursorSelections = TextSelectionSet.withPrimary(
		cursorGraphemes.map(grapheme => TextSelection.collapsedAt(grapheme.position)),
		controller.selections.primaryIndex,
	);
	const geometry = createStanzaVisualSelectionGeometry(
		overlay.model,
		cursorSelections,
		overlay.visualLineProjection,
		overlay.renderLines,
		overlay.textLeft,
		overlay.textMeasurer,
	);
	for (const rectangle of geometry.carets) {
		if (domCarets.has(rectangle.selectionIndex)) continue;
		appendCaret({
			renderingContext,
			overlay,
			host,
			cursors,
			renderedCursorIndexes,
			cursorOptions,
			lineHeight,
			selectionIndex: rectangle.selectionIndex,
			visualLineIndex: rectangle.visualLineIndex,
			caretLeft: rectangle.left,
			primary: rectangle.primary,
			hasMultipleCursors: cursorGraphemes.length > 1,
			grapheme: cursorGraphemes[rectangle.selectionIndex]!,
			pauseMovementAnimation,
			semanticTokenSource,
		});
	}
	for (const [selectionIndex, rectangle] of domCarets) {
		appendCaret({
			renderingContext,
			overlay,
			host,
			cursors,
			renderedCursorIndexes,
			cursorOptions,
			lineHeight,
			selectionIndex,
			visualLineIndex: rectangle.visualLineIndex,
			caretLeft: rectangle.left,
			domGeometry: rectangle,
			primary: selectionIndex === controller.selections.primaryIndex,
			hasMultipleCursors: cursorGraphemes.length > 1,
			grapheme: cursorGraphemes[selectionIndex]!,
			pauseMovementAnimation,
			semanticTokenSource,
		});
	}
	return renderedCursorIndexes;
}

interface DomCaretGeometry extends EditorVisiblePosition {
	readonly characterRange?: EditorLineVisibleRange;
}

function domCaretGeometry(context: EditorOverlayContext, position: TextPosition): DomCaretGeometry | undefined {
	const caret = context.visibleRangeForPosition(position);
	if (!caret) return undefined;
	const line = context.model.getLineContent(position.lineIndex);
	const nextBoundary = getTextGraphemeBoundaries(line).find(boundary => boundary > position.columnIndex);
	if (nextBoundary === undefined) return caret;
	const ranges = context.linesVisibleRangesForRange(TextRange.from(position, TextPosition.at(position.lineIndex, nextBoundary)), false);
	const characterRange = ranges?.find(range => range.visualLineIndex === caret.visualLineIndex);
	return characterRange ? Object.freeze({ ...caret, characterRange }) : caret;
}

interface AppendCaretOptions {
	readonly renderingContext: EditorRenderingContext;
	readonly overlay: EditorOverlayContext;
	readonly host: HTMLElement;
	readonly cursors: Map<number, ViewCursor>;
	readonly renderedCursorIndexes: Set<number>;
	readonly cursorOptions: ViewCursorOptions;
	readonly lineHeight: number;
	readonly selectionIndex: number;
	readonly visualLineIndex: number;
	readonly caretLeft: number;
	readonly domGeometry?: DomCaretGeometry;
	readonly primary: boolean;
	readonly hasMultipleCursors: boolean;
	readonly grapheme: CursorGrapheme;
	readonly pauseMovementAnimation: boolean;
	readonly semanticTokenSource: SemanticTokenSource | undefined;
}

function appendCaret(options: AppendCaretOptions): void {
	const {
		renderingContext,
		overlay,
		host,
		cursors,
		renderedCursorIndexes,
		cursorOptions,
		lineHeight,
		selectionIndex,
		visualLineIndex,
		caretLeft,
		domGeometry,
		primary,
		hasMultipleCursors,
		grapheme,
		pauseMovementAnimation,
		semanticTokenSource,
	} = options;
	const cursor = cursors.get(selectionIndex) ?? new ViewCursor(host, selectionIndex, cursorOptions);
	cursors.set(selectionIndex, cursor);
	const characterWidth = domGeometry?.characterRange?.width ?? cursorCharacterWidth(overlay, grapheme);
	const characterLeft = domGeometry?.characterRange?.left ?? (domGeometry?.isRightToLeft ? caretLeft - characterWidth : caretLeft);
	const plurality: ViewCursorPlurality = hasMultipleCursors ? primary ? 'primary' : 'secondary' : 'single';
	cursor.render({
		top: renderingContext.viewportData.getLineTop(visualLineIndex),
		caretLeft,
		characterLeft,
		characterWidth,
		character: grapheme.character,
		rowHeight: lineHeight,
		plurality,
		pauseMovementAnimation,
		presentation: cursorCharacterPresentation(semanticTokenSource, grapheme.position),
	});
	renderedCursorIndexes.add(selectionIndex);
}

function cursorCharacterPresentation(source: SemanticTokenSource | undefined, position: TextPosition): ViewCursorCharacterPresentation | undefined {
	const token = source?.getLineTokens(position.lineIndex).find(candidate => candidate.startColumn <= position.columnIndex && position.columnIndex < candidate.endColumn);
	if (!token) return undefined;
	const syntaxFontStyle = token.syntaxPresentation?.fontStyle ?? [];
	const decorations = syntaxFontStyle.filter(style => style === 'underline' || style === 'strikethrough').map(style => style === 'strikethrough' ? 'line-through' : style);
	return Object.freeze({
		classNames: Object.freeze(['stanza-editor-token', ...(token.presentation ? [token.presentation] : []), ...(token.modifiers ?? [])]),
		...(syntaxFontStyle.includes('italic') ? { fontStyle: 'italic' } : {}),
		...(syntaxFontStyle.includes('bold') ? { fontWeight: 'bold' } : {}),
		...(decorations.length > 0 ? { textDecorationLine: decorations.join(' ') } : {}),
	});
}

interface CursorGrapheme {
	readonly position: TextPosition;
	readonly character: string;
}

function cursorGrapheme(context: EditorOverlayContext, position: TextPosition): CursorGrapheme {
	const line = context.model.getLineContent(position.lineIndex);
	const boundaries = getTextGraphemeBoundaries(line);
	let startColumn = 0;
	for (const boundary of boundaries) {
		if (boundary > position.columnIndex) break;
		startColumn = boundary;
	}
	const nextBoundary = boundaries.find(boundary => boundary > startColumn);
	return Object.freeze({
		position: TextPosition.at(position.lineIndex, startColumn),
		character: nextBoundary === undefined ? '\u00a0' : line.slice(startColumn, nextBoundary),
	});
}

function cursorCharacterWidth(context: EditorOverlayContext, grapheme: CursorGrapheme): number {
	const position = grapheme.position;
	const line = context.model.getLineContent(position.lineIndex);
	const visualLineIndex = context.visualLineProjection.visualLineIndexAt(position);
	const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
	const startColumn = visualLine?.logicalLineIndex === position.lineIndex ? visualLine.startColumn : 0;
	const prefix = line.slice(startColumn, position.columnIndex);
	const throughCursor = grapheme.character === '\u00a0' ? `${prefix} ` : `${prefix}${grapheme.character}`;
	return Math.max(1, context.textMeasurer.measureLineWidth(throughCursor) - context.textMeasurer.measureLineWidth(prefix));
}

function projectStanzaCompositionOverlay(context: EditorOverlayContext, range: TextRange | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	if (!range) return;
	const rectangles = context.linesVisibleRangesForRange(range, false)
		?? createStanzaVisualRangeRectangles(context.model, [{ range, value: undefined }], context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of rectangles) {
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, 'div');
		element.className = 'stanza-editor-composition';
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		row.append(element);
	}
}
