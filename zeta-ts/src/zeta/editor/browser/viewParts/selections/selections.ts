import "./selections.css";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { h, reset } from '../../../../base/browser/dom.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { type EditorLineVisibleRange, type EditorOverlayContext } from '../../view/renderingContext.js';
import { EditorDynamicViewOverlay } from '../../view/editorDynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsOverlay extends EditorDynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly selectionController: CursorsController | undefined;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, selectionController: CursorsController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-selections-layer', 'stanza-editor-line-selections'));
		this.domNode = this.rows.domNode;
		this.selectionController = selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const rows = this.rows.render(context);
		projectStanzaSelectionOverlays(overlay, this.selectionController, rows);
	}
}

function projectStanzaSelectionOverlays(context: EditorOverlayContext, controller: CursorsController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	if (!controller) return;
	const domSelections = new Map<number, readonly EditorLineVisibleRange[]>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const selection = controller.selections.selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		const ranges = context.linesVisibleRangesForRange(selection, true);
		if (ranges) domSelections.set(selectionIndex, ranges);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.selections) {
		if (domSelections.has(rectangle.selectionIndex)) continue;
		appendSelection(context, rows, rectangle.selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle.width);
	}
	for (const [selectionIndex, ranges] of domSelections) {
		for (const rectangle of ranges) appendSelection(context, rows, selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle.width);
	}
}

function appendSelection(context: EditorOverlayContext, rows: ReadonlyMap<number, HTMLElement>, selectionIndex: number, visualLineIndex: number, left: number, width: number): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const element = h(context.ownerDocument, 'div');
	element.className = 'stanza-editor-selection';
	element.dataset.selectionIndex = String(selectionIndex);
	element.style.left = `${left}px`;
	element.style.width = `${width}px`;
	row.append(element);
}
