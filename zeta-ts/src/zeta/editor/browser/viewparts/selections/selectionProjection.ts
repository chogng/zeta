import { h, reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createStanzaVisualSelectionGeometry } from "../../../common/viewModel/visualSelectionGeometry.js";
import { createStanzaDomSelectionGeometry, type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

export function projectStanzaCurrentLineHighlight(context: ViewportOverlayContext, controller: EditorSelectionController | undefined): void {
	const activeLineIndex = controller?.selections.primary.active.lineIndex;
	for (const [visualLineIndex, line] of context.renderedLines) {
		const active = context.activeLineHighlight === "on" && context.visualLineProjection.lineAt(visualLineIndex)?.logicalLineIndex === activeLineIndex;
		line.numberDomNode.setClassName(active ? "stanza-editor-line-number active" : "stanza-editor-line-number");
		line.domNode.setClassName(active ? "stanza-editor-line active" : "stanza-editor-line");
	}
}

export function projectStanzaSelectionOverlays(context: ViewportOverlayContext, controller: EditorSelectionController | undefined): void {
	for (const line of context.renderedLines.values()) reset(line.selectionElement);
	if (!controller) return;
	const domGeometry = context.useDomTextGeometry ? createStanzaDomSelectionGeometry(context, controller.selections) : undefined;
	const geometry = createStanzaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	for (const rectangle of geometry.selections) {
		if (domGeometry?.selectionIndexes.has(rectangle.selectionIndex)) continue;
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-selection";
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		line.selectionElement.append(element);
	}
	for (const rectangle of domGeometry?.selections ?? []) {
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(context.ownerDocument, "div");
		element.className = "stanza-editor-selection";
		element.dataset.selectionIndex = String(rectangle.selectionIndex);
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		line.selectionElement.append(element);
	}
}
