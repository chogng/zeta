import { h, reset } from "../../../../base/browser/dom.js";
import { createStanzaVisualDecorationRectangles, DecorationPresentation, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext, createStanzaDomRangeRectangles } from "../viewportOverlay/viewportOverlayPresentation.js";

/** Projects visible inline decorations into rows. */
export function projectStanzaDecorationOverlays(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[], rows: ReadonlyMap<number, HTMLElement>): void {
	const inlineDecorations = decorations.filter(decoration => (
		decoration.presentation !== DecorationPresentation.GlyphMargin
		&& decoration.presentation !== DecorationPresentation.LineDecoration
	));
	const rectangles = createStanzaVisualDecorationRectangles(context.model, inlineDecorations, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	const domRectangles = context.useDomTextGeometry
		? new Map(inlineDecorations.map(decoration => [decoration.id, createStanzaDomRangeRectangles(context, decoration.range)] as const))
		: undefined;
	for (const row of rows.values()) reset(row);
	const ownerDocument = context.ownerDocument;
	for (const rectangle of rectangles) {
		if (domRectangles?.get(rectangle.id)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		const element = h(ownerDocument, "div");
		element.className = "stanza-editor-decoration";
		element.classList.add(rectangle.presentation);
		element.dataset.decorationId = String(rectangle.id);
		if (rectangle.hoverText !== undefined) element.title = rectangle.hoverText;
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		row.append(element);
	}
	for (const decoration of inlineDecorations) {
		const geometry = domRectangles?.get(decoration.id);
		if (!geometry) continue;
		for (const rectangle of geometry) {
			const row = rows.get(rectangle.visualLineIndex);
			if (!row) continue;
			const element = h(ownerDocument, "div");
			element.className = "stanza-editor-decoration";
			element.classList.add(decoration.presentation);
			element.dataset.decorationId = String(decoration.id);
			if (decoration.hoverText !== undefined) element.title = decoration.hoverText;
			element.style.left = `${rectangle.left}px`;
			element.style.width = `${rectangle.width}px`;
			row.append(element);
		}
	}
}
