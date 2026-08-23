import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createAsterVisualSelectionGeometry } from "../../../common/viewModel/visualSelectionGeometry.js";
import { createAsterDomSelectionGeometry, type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

export function projectAsterCursorOverlays(context: ViewportOverlayContext, controller: EditorSelectionController | undefined): void {
	for (const line of context.renderedLines.values()) reset(line.cursorElement);
	if (!controller) return;
	const domGeometry = context.useDomTextGeometry ? createAsterDomSelectionGeometry(context, controller.selections) : undefined;
	const geometry = createAsterVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.carets) {
		if (domGeometry?.caretIndexes.has(rectangle.selectionIndex)) continue;
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "aster-editor-caret";
		element.classList.toggle("primary", rectangle.primary);
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		line.cursorElement.append(element);
	}
	for (const rectangle of domGeometry?.carets ?? []) {
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "aster-editor-caret";
		element.classList.toggle("primary", rectangle.primary);
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		line.cursorElement.append(element);
	}
}
