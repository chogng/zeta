import "./selections.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { type Selection } from '../../../common/core/selection.js';
import { type IViewModel } from '../../../common/viewModel.js';
import { type EditorLineVisibleRange, type EditorOverlayContext } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext } from "../../view/viewPart.js";
import { type ViewContext } from '../../../common/viewModel/viewContext.js';

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsOverlay extends DynamicViewOverlay {
	constructor(private readonly context: ViewContext, private readonly viewModel: IViewModel) {
		super();
		this.context.addEventHandler(this);
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: EditorRenderingContext): void {
		this.prepareRows(context, (overlay, rows) => {
			projectStanzaSelectionOverlays(overlay, this.viewModel.getCursorStates().map(state => state.modelState.selection), rows);
		});
	}
}

function projectStanzaSelectionOverlays(context: EditorOverlayContext, selections: readonly Selection[], rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	const domSelections = new Map<number, readonly EditorLineVisibleRange[]>();
	for (let selectionIndex = 0; selectionIndex < selections.length; selectionIndex += 1) {
		const selection = selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		const ranges = context.linesVisibleRangesForRange(selection, true);
		if (ranges) domSelections.set(selectionIndex, ranges);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
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
	element.className = 'cslr selected-text stanza-editor-selection';
	element.dataset.selectionIndex = String(selectionIndex);
	element.style.left = `${left}px`;
	element.style.width = `${width}px`;
	row.append(element);
}
