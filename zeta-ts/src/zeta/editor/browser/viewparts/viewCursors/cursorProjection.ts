import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createStanzaVisualSelectionGeometry } from "../../../common/viewModel/visualSelectionGeometry.js";
import { createStanzaDomSelectionGeometry, type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

export function projectStanzaCursorOverlays(context: ViewportOverlayContext, controller: EditorSelectionController | undefined, rows: ReadonlyMap<number, HTMLElement>): void {
	for (const row of rows.values()) reset(row);
	if (!controller) return;
	const domGeometry = context.useDomTextGeometry ? createStanzaDomSelectionGeometry(context, controller.selections) : undefined;
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domGeometry?.caretIndexes.has(rectangle.selectionIndex)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-caret";
		element.classList.toggle("primary", rectangle.primary);
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		row.append(element);
	}
	for (const rectangle of domGeometry?.carets ?? []) {
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-caret";
		element.classList.toggle("primary", rectangle.primary);
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		row.append(element);
	}
}
