import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createStanzaVisualSelectionGeometry } from "../../../common/viewModel/visualSelectionGeometry.js";
import { type EditorLineVisibleRange, type EditorOverlayContext } from "../../view/renderingContext.js";

export function projectStanzaCurrentLineHighlight(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	const activeLineIndex = controller?.selections.primary.active.lineIndex;
	for (const [visualLineIndex, row] of rows) {
		const active = context.activeLineHighlight === "on" && context.visualLineProjection.lineAt(visualLineIndex)?.logicalLineIndex === activeLineIndex;
		row.classList.toggle('active', active);
	}
}

export function projectStanzaSelectionOverlays(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	if (!controller) return;
	const domSelections = new Map<number, readonly EditorLineVisibleRange[]>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const selection = controller.selections.selections[selectionIndex]!;
		if (selection.collapsed) continue;
		const ranges = context.linesVisibleRangesForRange(selection.range, true);
		if (ranges) domSelections.set(selectionIndex, ranges);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.selections) {
		if (domSelections.has(rectangle.selectionIndex)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-selection";
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		row.append(element);
	}
	for (const [selectionIndex, ranges] of domSelections) {
		for (const rectangle of ranges) {
			const row = rows.get(rectangle.visualLineIndex);
			if (!row) continue;
			const element = h(context.ownerDocument, "div");
			element.className = "stanza-editor-selection";
			element.dataset.selectionIndex = String(selectionIndex);
			element.style.left = `${rectangle.left}px`;
			element.style.width = `${rectangle.width}px`;
			row.append(element);
		}
	}
}
