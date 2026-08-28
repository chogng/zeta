import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createStanzaVisualSelectionGeometry } from "../../../common/viewModel/visualSelectionGeometry.js";
import { type EditorOverlayContext, type EditorVisiblePosition } from "../../view/renderingContext.js";

export function projectStanzaCursorOverlays(context: EditorOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	if (!controller) return;
	const domCarets = new Map<number, EditorVisiblePosition>();
	for (let selectionIndex = 0; selectionIndex < controller.selections.selections.length; selectionIndex += 1) {
		const position = context.visibleRangeForPosition(controller.selections.selections[selectionIndex]!.active);
		if (position) domCarets.set(selectionIndex, position);
	}
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domCarets.has(rectangle.selectionIndex)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-caret";
		element.classList.toggle("primary", rectangle.primary);
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		row.append(element);
	}
	for (const [selectionIndex, rectangle] of domCarets) {
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-caret";
		element.classList.toggle("primary", selectionIndex === controller.selections.primaryIndex);
		element.dataset.selectionIndex = String(selectionIndex);
		element.style.left = `${rectangle.left}px`;
		row.append(element);
	}
}
