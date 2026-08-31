import "./selections.css";
import { h, reset } from '../../../../base/browser/dom.js';
import { createStanzaVisualSelectionGeometry } from '../../../common/viewModel/visualSelectionGeometry.js';
import { type Selection } from '../../../common/core/selection.js';
import { type IViewModel } from '../../../common/viewModel.js';
import { type LineVisibleRanges, type RenderingContext } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type TextMeasurer } from '../../../common/viewModel/textMeasurer.js';
import { type TextModel } from '../../../common/model/textModel.js';

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsOverlay extends DynamicViewOverlay {
	constructor(
		private readonly context: ViewContext,
		private readonly viewModel: IViewModel,
		private readonly model: TextModel,
		private readonly ownerDocument: Document,
		private readonly readVisualProjection: () => EditorVisualLineProjection,
		private readonly readTextLeft: () => number,
		private readonly textMeasurer: TextMeasurer,
	) {
		super();
		this.context.addEventHandler(this);
	}

	public override dispose(): void {
		this.context.removeEventHandler(this);
		super.dispose();
	}

	public prepareRender(context: RenderingContext): void {
		this.prepareRows(context, this.ownerDocument, rows => {
			projectStanzaSelectionOverlays(
				context,
				this.model,
				this.readVisualProjection(),
				this.readTextLeft(),
				this.textMeasurer,
				this.viewModel.getCursorStates().map(state => state.modelState.selection),
				rows,
			);
		});
	}
}

function projectStanzaSelectionOverlays(context: RenderingContext, model: TextModel, projection: EditorVisualLineProjection, textLeft: number, textMeasurer: TextMeasurer, selections: readonly Selection[], rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	const domSelections = new Map<number, readonly LineVisibleRanges[]>();
	for (let selectionIndex = 0; selectionIndex < selections.length; selectionIndex += 1) {
		const selection = selections[selectionIndex]!;
		if (selection.isEmpty()) continue;
		const ranges = context.linesVisibleRangesForRange(selection, true);
		if (ranges) domSelections.set(selectionIndex, ranges);
	}
	const renderLines = { startLineIndex: context.viewportData.startLineNumber - 1, endLineIndexExclusive: context.viewportData.endLineNumber };
	const geometry = createStanzaVisualSelectionGeometry(model, selections, projection, renderLines, textLeft, textMeasurer);
	for (const rectangle of geometry.selections) {
		if (domSelections.has(rectangle.selectionIndex)) continue;
		appendSelection(rows, rectangle.selectionIndex, rectangle.visualLineIndex, rectangle.left, rectangle.width);
	}
	for (const [selectionIndex, ranges] of domSelections) {
		for (const line of ranges) {
			for (const range of line.ranges) appendSelection(rows, selectionIndex, line.lineNumber - 1, range.left, range.width);
		}
	}
}

function appendSelection(rows: ReadonlyMap<number, HTMLElement>, selectionIndex: number, visualLineIndex: number, left: number, width: number): void {
	const row = rows.get(visualLineIndex);
	if (!row) return;
	const element = h(row.ownerDocument, 'div');
	element.className = 'cslr selected-text stanza-editor-selection';
	element.dataset.selectionIndex = String(selectionIndex);
	element.style.left = `${left}px`;
	element.style.width = `${width}px`;
	row.append(element);
}
