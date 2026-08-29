import "./viewCursors.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextPosition, type TextRange } from '../../../common/core/text.js';
import { getTextGraphemeBoundaries } from '../../../common/core/textSegmentation.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../../../common/model/trackedRange.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorOverlayContext, type EditorVisiblePosition } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';
import { ViewCursor, type ViewCursorOptions } from './viewCursor.js';

export interface ViewCursorsOptions extends ViewCursorOptions {
	readonly host: HTMLElement;
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
}

function projectStanzaCursorOverlays(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>, cursors: Map<number, ViewCursor>, cursorOptions: ViewCursorOptions, lineHeight: number): ReadonlySet<number> {
	const renderedCursorIndexes = new Set<number>();
	if (!controller) return renderedCursorIndexes;
	const domCarets = new Map<number, EditorVisiblePosition>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const position = context.visibleRangeForPosition(controller.selections.selections[selectionIndex]!.active);
		if (position) domCarets.set(selectionIndex, position);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domCarets.has(rectangle.selectionIndex)) continue;
		appendCaret(context, rows, cursors, renderedCursorIndexes, cursorOptions, lineHeight, rectangle.selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle.primary, controller.selections.selections[rectangle.selectionIndex]!.active);
	}
	for (const [selectionIndex, rectangle] of domCarets) {
		appendCaret(context, rows, cursors, renderedCursorIndexes, cursorOptions, lineHeight, selectionIndex, rectangle.visualLineIndex, rectangle.left, selectionIndex === controller.selections.primaryIndex, controller.selections.selections[selectionIndex]!.active);
	}
	return renderedCursorIndexes;
}

function appendCaret(context: EditorOverlayContext, rows: ReadonlyMap<number, HTMLElement>, cursors: Map<number, ViewCursor>, renderedCursorIndexes: Set<number>, cursorOptions: ViewCursorOptions, lineHeight: number, selectionIndex: number, visualLineIndex: number, left: number, primary: boolean, position: TextPosition): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const cursor = cursors.get(selectionIndex) ?? new ViewCursor(row, selectionIndex, cursorOptions);
	cursors.set(selectionIndex, cursor);
	cursor.render(row, left, cursorCharacterWidth(context, position), lineHeight, primary);
	renderedCursorIndexes.add(selectionIndex);
}

function cursorCharacterWidth(context: EditorOverlayContext, position: TextPosition): number {
	const line = context.model.getLineContent(position.lineIndex);
	const nextBoundary = getTextGraphemeBoundaries(line).find(boundary => boundary > position.columnIndex) ?? position.columnIndex;
	const nextCharacter = line.slice(position.columnIndex, nextBoundary);
	return Math.max(1, context.textMeasurer.measureLineWidth(nextCharacter || ' '));
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
