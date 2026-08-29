import "./viewCursors.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextRange } from '../../../common/core/text.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { TrackedRangeStickiness, type TrackedRange } from '../../../common/model/trackedRange.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { createStanzaVisualRangeRectangles } from '../../../common/viewModel/visualRangeGeometry.js';
import { type EditorOverlayContext, type EditorVisiblePosition } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursors extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly model: TextModel;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly rows: ViewPartRows;
	private compositionRange: TrackedRange | undefined;

	constructor(context: EditorViewContext, host: HTMLElement, model: TextModel, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-cursors-layer', 'stanza-editor-line-cursors'));
		this.domNode = this.rows.domNode;
		this.model = model;
		this.selectionController = selectionController;
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
		projectStanzaCursorOverlays(overlay, this.selectionController, rows);
	}
}

function projectStanzaCursorOverlays(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	if (!controller) return;
	const domCarets = new Map<number, EditorVisiblePosition>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const position = context.visibleRangeForPosition(controller.selections.selections[selectionIndex]!.active);
		if (position) domCarets.set(selectionIndex, position);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domCarets.has(rectangle.selectionIndex)) continue;
		appendCaret(context, rows, rectangle.selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle.primary);
	}
	for (const [selectionIndex, rectangle] of domCarets) {
		appendCaret(context, rows, selectionIndex, rectangle.visualLineIndex, rectangle.left, selectionIndex === controller.selections.primaryIndex);
	}
}

function appendCaret(context: EditorOverlayContext, rows: ReadonlyMap<number, HTMLElement>, selectionIndex: number, visualLineIndex: number, left: number, primary: boolean): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const element = h(context.ownerDocument, 'div');
	element.className = 'stanza-editor-caret';
	element.classList.toggle('primary', primary);
	element.dataset.selectionIndex = String(selectionIndex);
	element.style.left = `${left}px`;
	row.append(element);
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
