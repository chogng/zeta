import { h, reset } from "../../../../base/browser/dom.js";
import { createStanzaVisualDecorationRectangles, DecorationPresentation, type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type EditorOverlayContext } from "../../view/renderingContext.js";

/** Projects visible inline decorations into rows. */
export function projectStanzaDecorationOverlays(context: EditorOverlayContext, decorations: readonly ResolvedDecoration[], rows: ReadonlyMap<number, HTMLElement>): void {
	const inlineDecorations = decorations.filter(decoration => (
		decoration.presentation !== DecorationPresentation.GlyphMargin
		&& decoration.presentation !== DecorationPresentation.LineDecoration
	));
	const rectangles = createStanzaVisualDecorationRectangles(context.model, inlineDecorations, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	const domRectangles = new Map(inlineDecorations.map(decoration => [decoration.id, context.linesVisibleRangesForRange(decoration.range, false)] as const));
	const decorationsById = new Map(inlineDecorations.map(decoration => [decoration.id, decoration] as const));
	for (const row of rows.values()) reset(row);
	const ownerDocument = context.ownerDocument;
	for (const rectangle of rectangles) {
		if (domRectangles?.get(rectangle.id)) continue;
		const row = rows.get(rectangle.visualLineIndex);
		if (!row) continue;
		row.append(createDecorationElement(ownerDocument, decorationsById.get(rectangle.id)!, rectangle.left, rectangle.width));
	}
	for (const decoration of inlineDecorations) {
		const geometry = domRectangles?.get(decoration.id);
		if (!geometry) continue;
		for (const rectangle of geometry) {
			const row = rows.get(rectangle.visualLineIndex);
			if (!row) continue;
			row.append(createDecorationElement(ownerDocument, decoration, rectangle.left, rectangle.width));
		}
	}
}

function createDecorationElement(ownerDocument: Document, decoration: ResolvedDecoration, left: number, width: number): HTMLElement {
	const element = h(ownerDocument, 'div');
	element.className = 'stanza-editor-decoration';
	element.classList.add(decoration.presentation);
	element.dataset.decorationId = String(decoration.id);
	if (decoration.hoverText !== undefined) element.title = decoration.hoverText;
	if (decoration.presentation === DecorationPresentation.ColorSwatch) {
		element.setAttribute('role', 'button');
		element.setAttribute('aria-label', decoration.hoverText ?? 'Edit color');
		element.tabIndex = -1;
		element.style.setProperty('--stanza-editor-color-swatch', decoration.color!);
		element.style.left = `${left - 14}px`;
		element.style.width = '10px';
	} else {
		element.style.left = `${left}px`;
		element.style.width = `${width}px`;
	}
	return element;
}
