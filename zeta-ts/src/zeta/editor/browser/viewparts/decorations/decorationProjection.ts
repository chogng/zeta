import { h, reset } from "../../../../base/browser/dom.js";
import { createAsterVisualDecorationRectangles, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext, createAsterDomRangeRectangles } from "../viewportOverlay/viewportOverlayPresentation.js";

/** Projects visible inline decorations into rows. */
export function projectAsterDecorationOverlays(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[]): void {
	const rectangles = createAsterVisualDecorationRectangles(context.model, decorations, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	const domRectangles = context.useDomTextGeometry
		? new Map(decorations.map(decoration => [decoration.id, createAsterDomRangeRectangles(context, decoration.range)] as const))
		: undefined;
	for (const line of context.renderedLines.values()) reset(line.decorationElement);
	const ownerDocument = context.ownerDocument;
	for (const rectangle of rectangles) {
		if (domRectangles?.get(rectangle.id)) continue;
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(ownerDocument, "div");
		element.className = "aster-editor-decoration";
		element.classList.add(rectangle.presentation);
		element.dataset.decorationId = String(rectangle.id);
		if (rectangle.hoverText !== undefined) element.title = rectangle.hoverText;
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		line.decorationElement.append(element);
	}
	for (const decoration of decorations) {
		const geometry = domRectangles?.get(decoration.id);
		if (!geometry) continue;
		for (const rectangle of geometry) {
			const line = context.renderedLines.get(rectangle.visualLineIndex);
			if (!line) continue;
			const element = h(ownerDocument, "div");
			element.className = "aster-editor-decoration";
			element.classList.add(decoration.presentation);
			element.dataset.decorationId = String(decoration.id);
			if (decoration.hoverText !== undefined) element.title = decoration.hoverText;
			element.style.left = `${rectangle.left}px`;
			element.style.width = `${rectangle.width}px`;
			line.decorationElement.append(element);
		}
	}
}
