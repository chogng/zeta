import { reset } from "../../../base/browser/dom.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { type TextRange } from "../common/text.js";
import { type TextModel } from "../common/textModel.js";
import { type EditorLineRange } from "../common/viewport.js";
import { createAlphaDecorationRectangles, type AlphaResolvedDecoration } from "./decorationPresentation.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { createAlphaRangeRectangles } from "./rangeGeometry.js";
import { type AlphaRenderedLine } from "./renderedLine.js";
import { createAlphaSelectionGeometry } from "./selectionGeometry.js";

export interface AlphaViewportOverlayContext {
  readonly ownerDocument: Document;
  readonly model: TextModel;
  readonly renderedLines: ReadonlyMap<number, AlphaRenderedLine>;
  readonly renderLines: EditorLineRange;
  readonly textLeft: number;
  readonly textMeasurer: AlphaTextMeasurer;
}

export function projectAlphaSelectionOverlays(context: AlphaViewportOverlayContext, controller: EditorSelectionController | undefined): void {
  const activeLineIndex = controller?.selections.primary.active.lineIndex;
  for (const [lineIndex, line] of context.renderedLines) {
    reset(line.selectionElement);
    line.numberElement.classList.toggle("active", lineIndex === activeLineIndex);
  }
  if (!controller) return;

  const geometry = createAlphaSelectionGeometry(context.model, controller.selections, context.renderLines, context.textLeft, context.textMeasurer);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of geometry.selections) {
    const line = context.renderedLines.get(rectangle.lineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-selection";
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.selectionElement.append(element);
  }
  for (const rectangle of geometry.carets) {
    const line = context.renderedLines.get(rectangle.lineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-caret";
    element.classList.toggle("primary", rectangle.primary);
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    line.selectionElement.append(element);
  }
}

export function projectAlphaDecorationOverlays(context: AlphaViewportOverlayContext, decorations: readonly AlphaResolvedDecoration[]): void {
  const rectangles = createAlphaDecorationRectangles(context.model, decorations, context.renderLines, context.textLeft, context.textMeasurer);
  for (const line of context.renderedLines.values()) reset(line.decorationElement);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of rectangles) {
    const line = context.renderedLines.get(rectangle.lineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-decoration";
    element.classList.add(rectangle.presentation);
    element.dataset.decorationId = String(rectangle.id);
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.decorationElement.append(element);
  }
}

export function projectAlphaCompositionOverlay(context: AlphaViewportOverlayContext, range: TextRange | undefined): void {
  for (const line of context.renderedLines.values()) reset(line.compositionElement);
  if (!range) return;
  const rectangles = createAlphaRangeRectangles(context.model, [{ range, value: undefined }], context.renderLines, context.textLeft, context.textMeasurer);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of rectangles) {
    const line = context.renderedLines.get(rectangle.lineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-composition";
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.compositionElement.append(element);
  }
}
