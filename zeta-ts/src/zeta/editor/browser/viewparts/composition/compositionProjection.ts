import { h, reset } from "../../../../base/browser/dom.js";
import { type TextRange } from "../../../common/core/text.js";
import { createAsterVisualRangeRectangles } from "../../../common/viewModel/visualRangeGeometry.js";
import { createAsterDomRangeRectangles, type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

/** Projects the current IME range into the reusable row composition layers. */
export function projectAsterCompositionOverlay(context: ViewportOverlayContext, range: TextRange | undefined): void {
	for (const line of context.renderedLines.values()) reset(line.compositionElement);
	if (!range) return;
	const domRectangles = context.useDomTextGeometry ? createAsterDomRangeRectangles(context, range) : undefined;
	const rectangles = domRectangles ?? createAsterVisualRangeRectangles(context.model, [{ range, value: undefined }], context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
	const ownerDocument = context.ownerDocument;
	for (const rectangle of rectangles) {
		const line = context.renderedLines.get(rectangle.visualLineIndex);
		if (!line) continue;
		const element = h(ownerDocument, "div");
		element.className = "aster-editor-composition";
		element.style.left = `${rectangle.left}px`;
		element.style.width = `${rectangle.width}px`;
		line.compositionElement.append(element);
	}
}
