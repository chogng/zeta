import "./viewCursors.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { TextEditorCursorBlinkingStyle } from '../../../common/config/editorOptions.js';
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextPosition, TextRange } from '../../../common/core/text.js';
import { getTextGraphemeBoundaries } from '../../../common/core/textSegmentation.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../../../common/model/trackedRange.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorLineVisibleRange, type EditorOverlayContext, type EditorVisiblePosition } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';
import { ViewCursor, type ViewCursorOptions } from './viewCursor.js';

export interface ViewCursorsOptions extends ViewCursorOptions {
	readonly host: HTMLElement;
	readonly blinking: TextEditorCursorBlinkingStyle;
}

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursors extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly model: TextModel;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly rows: ViewPartRows;
	private readonly cursorOptions: ViewCursorOptions;
	private readonly cursors = new Map<number, ViewCursor>();
	private compositionRange: TrackedRange | undefined;

	constructor(context: EditorViewContext, options: ViewCursorsOptions, model: TextModel, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(options.host, 'stanza-editor-cursors-layer', 'stanza-editor-line-cursors'));
		this.domNode = this.rows.domNode;
		this.domNode.classList.add(cursorBlinkingClass(options.blinking));
		this.model = model;
		this.selectionController = selectionController;
		this.cursorOptions = options;
		this._register(toDisposable(() => this.compositionRange?.dispose()));
	}

	public setCompositionRange(range: TextRange | undefined): void {
		const next = range ? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges) : undefined;
		this.compositionRange?.dispose();
		this.compositionRange = next;
		this.renderNow(this.context.renderingContext);
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const rows = this.rows.render(context);
		for (const row of rows.values()) reset(row);
		projectStanzaCompositionOverlay(overlay, this.compositionRange?.range, rows);
		const renderedCursorIndexes = projectStanzaCursorOverlays(overlay, this.selectionController, rows, this.cursors, this.cursorOptions, context.layout.lineHeight);
		for (const [selectionIndex, cursor] of this.cursors) {
			if (renderedCursorIndexes.has(selectionIndex)) {
				continue;
			}
			cursor.domNode.remove();
			this.cursors.delete(selectionIndex);
		}
	}

	public renderSelection(context: EditorRenderingContext): void {
		this.renderNow(context);
		for (const animation of this.domNode.getAnimations?.() ?? []) animation.currentTime = 0;
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

function projectStanzaCursorOverlays(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>, cursors: Map<number, ViewCursor>, cursorOptions: ViewCursorOptions, lineHeight: number): ReadonlySet<number> {
	const renderedCursorIndexes = new Set<number>();
	if (!controller) return renderedCursorIndexes;
	const domCarets = new Map<number, DomCaretGeometry>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const geometry = domCaretGeometry(context, controller.selections.selections[selectionIndex]!.active);
		if (geometry) domCarets.set(selectionIndex, geometry);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domCarets.has(rectangle.selectionIndex)) continue;
		appendCaret(context, rows, cursors, renderedCursorIndexes, cursorOptions, lineHeight, rectangle.selectionIndex, rectangle.visualLineIndex, rectangle.left, undefined, rectangle.primary, controller.selections.selections[rectangle.selectionIndex]!.active);
	}
	for (const [selectionIndex, rectangle] of domCarets) {
		appendCaret(context, rows, cursors, renderedCursorIndexes, cursorOptions, lineHeight, selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle, selectionIndex === controller.selections.primaryIndex, controller.selections.selections[selectionIndex]!.active);
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

function appendCaret(context: EditorOverlayContext, rows: ReadonlyMap<number, HTMLElement>, cursors: Map<number, ViewCursor>, renderedCursorIndexes: Set<number>, cursorOptions: ViewCursorOptions, lineHeight: number, selectionIndex: number, visualLineIndex: number, caretLeft: number, domGeometry: DomCaretGeometry | undefined, primary: boolean, position: TextPosition): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const cursor = cursors.get(selectionIndex) ?? new ViewCursor(row, selectionIndex, cursorOptions);
	cursors.set(selectionIndex, cursor);
	const characterWidth = domGeometry?.characterRange?.width ?? cursorCharacterWidth(context, position);
	const characterLeft = domGeometry?.characterRange?.left ?? (domGeometry?.isRightToLeft ? caretLeft - characterWidth : caretLeft);
	cursor.render(row, caretLeft, characterLeft, characterWidth, lineHeight, primary);
	renderedCursorIndexes.add(selectionIndex);
}

function cursorCharacterWidth(context: EditorOverlayContext, position: TextPosition): number {
	const line = context.model.getLineContent(position.lineIndex);
	const nextBoundary = getTextGraphemeBoundaries(line).find(boundary => boundary > position.columnIndex) ?? position.columnIndex;
	const visualLineIndex = context.visualLineProjection.visualLineIndexAt(position);
	const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
	const startColumn = visualLine?.logicalLineIndex === position.lineIndex ? visualLine.startColumn : 0;
	const prefix = line.slice(startColumn, position.columnIndex);
	const throughCursor = nextBoundary > position.columnIndex
		? line.slice(startColumn, nextBoundary)
		: `${prefix} `;
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
