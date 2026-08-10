import { reset } from "../../../../base/browser/dom.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { type TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange } from "../../common/viewLayout/editorViewportModel.js";
import { DecorationPresentation, createAlphaVisualDecorationRectangles, type ResolvedDecoration } from "./decorationPresentation.js";
import { type TextMeasurer } from "./fontMetrics.js";
import { getAlphaDomTextCaretLeft, getAlphaDomTextRangeRectangles } from "./domTextGeometry.js";
import { type RenderedLine } from "./renderedLine.js";
import { createAlphaVisualRangeRectangles } from "./visualRangeGeometry.js";
import { createAlphaVisualSelectionGeometry } from "./visualSelectionGeometry.js";

const DIAGNOSTIC_PRESENTATION_PRIORITY = new Map<ResolvedDecoration["presentation"], number>([
  [DecorationPresentation.ErrorUnderline, 4],
  [DecorationPresentation.WarningUnderline, 3],
  [DecorationPresentation.InformationUnderline, 2],
  [DecorationPresentation.HintUnderline, 1],
]);

/** Selects whether selection projection marks the cursor's logical line as active. */
export type ActiveLineHighlight = "on" | "off";

export interface ViewportOverlayContext {
  readonly ownerDocument: Document;
  readonly model: TextModel;
  readonly visualLineProjection: EditorVisualLineProjection;
  readonly renderedLines: ReadonlyMap<number, RenderedLine>;
  readonly renderLines: EditorLineRange;
  readonly textLeft: number;
  readonly textMeasurer: TextMeasurer;
  /** Uses browser range geometry when text direction may produce non-monotonic advances. */
  readonly useDomTextGeometry: boolean;
  /** `off` matches simple input editors by omitting current-line presentation DOM. */
  readonly activeLineHighlight: ActiveLineHighlight;
}

export function projectAlphaSelectionOverlays(context: ViewportOverlayContext, controller: EditorSelectionController | undefined): void {
  const activeLineIndex = controller?.selections.primary.active.lineIndex;
  for (const [visualLineIndex, line] of context.renderedLines) {
    reset(line.selectionElement);
    const active = context.activeLineHighlight === "on" && context.visualLineProjection.lineAt(visualLineIndex)?.logicalLineIndex === activeLineIndex;
    line.numberElement.classList.toggle("active", active);
    line.element.classList.toggle("active", active);
  }
  if (!controller) return;

  const domGeometry = context.useDomTextGeometry
    ? createDomSelectionGeometry(context, controller.selections)
    : undefined;
  const geometry = createAlphaVisualSelectionGeometry(context.model, controller.selections, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of geometry.selections) {
    if (domGeometry?.selectionIndexes.has(rectangle.selectionIndex)) continue;
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-selection";
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.selectionElement.append(element);
  }
  for (const rectangle of domGeometry?.selections ?? []) {
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-selection";
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.selectionElement.append(element);
  }
  for (const rectangle of geometry.carets) {
    if (domGeometry?.caretIndexes.has(rectangle.selectionIndex)) continue;
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-caret";
    element.classList.toggle("primary", rectangle.primary);
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    line.selectionElement.append(element);
  }
  for (const rectangle of domGeometry?.carets ?? []) {
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-caret";
    element.classList.toggle("primary", rectangle.primary);
    element.dataset.selectionIndex = String(rectangle.selectionIndex);
    element.style.left = `${rectangle.left}px`;
    line.selectionElement.append(element);
  }
}

interface DomSelectionRectangle {
  readonly selectionIndex: number;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly width: number;
}

interface DomCaretRectangle {
  readonly selectionIndex: number;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly primary: boolean;
}

interface DomSelectionGeometry {
  readonly selectionIndexes: ReadonlySet<number>;
  readonly selections: readonly DomSelectionRectangle[];
  readonly caretIndexes: ReadonlySet<number>;
  readonly carets: readonly DomCaretRectangle[];
}

function createDomSelectionGeometry(context: ViewportOverlayContext, selections: EditorSelectionController["selections"]): DomSelectionGeometry | undefined {
  const selectionIndexes = new Set<number>();
  const domSelections: DomSelectionRectangle[] = [];
  const caretIndexes = new Set<number>();
  const domCarets: DomCaretRectangle[] = [];
  for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
    const selection = selections.selections[selectionIndex]!;
    if (!selection.collapsed) {
      const candidate = domRangeRectanglesForRange(context, selection.range);
      if (candidate) {
        selectionIndexes.add(selectionIndex);
        domSelections.push(...candidate.map(rectangle => Object.freeze({ ...rectangle, selectionIndex })));
      }
    }
    const visualLineIndex = context.visualLineProjection.visualLineIndexAt(selection.active);
    const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
    const renderedLine = context.renderedLines.get(visualLineIndex);
    if (!visualLine || !renderedLine) continue;
    const offset = selection.active.columnIndex - visualLine.startColumn;
    if (!isCurrentDomTextOffset(renderedLine.textElement, offset)) continue;
    const left = getAlphaDomTextCaretLeft(
      renderedLine.textElement,
      offset,
      renderedLine.element,
    );
    if (left === undefined) continue;
    caretIndexes.add(selectionIndex);
    domCarets.push(Object.freeze({
      selectionIndex,
      visualLineIndex,
      left,
      primary: selectionIndex === selections.primaryIndex,
    }));
  }
  if (selectionIndexes.size === 0 && caretIndexes.size === 0) return undefined;
  return Object.freeze({
    selectionIndexes,
    selections: Object.freeze(domSelections),
    caretIndexes,
    carets: Object.freeze(domCarets),
  });
}

interface DomVisualRangeRectangle {
  readonly visualLineIndex: number;
  readonly left: number;
  readonly width: number;
}

function domRangeRectanglesForRange(context: ViewportOverlayContext, range: TextRange): readonly DomVisualRangeRectangle[] | undefined {
  const result: DomVisualRangeRectangle[] = [];
  let intersectsRenderedLine = false;
  for (let visualLineIndex = context.renderLines.startLineIndex; visualLineIndex < context.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
    const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
    const renderedLine = context.renderedLines.get(visualLineIndex);
    if (!visualLine || !renderedLine || visualLine.logicalLineIndex < range.start.lineIndex || visualLine.logicalLineIndex > range.end.lineIndex) continue;
    const startColumn = visualLine.logicalLineIndex === range.start.lineIndex
      ? Math.max(visualLine.startColumn, range.start.columnIndex)
      : visualLine.startColumn;
    const endColumn = visualLine.logicalLineIndex === range.end.lineIndex
      ? Math.min(visualLine.endColumn, range.end.columnIndex)
      : visualLine.endColumn;
    if (endColumn <= startColumn) continue;
    intersectsRenderedLine = true;
    const startOffset = startColumn - visualLine.startColumn;
    const endOffset = endColumn - visualLine.startColumn;
    if (!isCurrentDomTextOffset(renderedLine.textElement, startOffset) || !isCurrentDomTextOffset(renderedLine.textElement, endOffset)) return undefined;
    const rectangles = getAlphaDomTextRangeRectangles(
      renderedLine.textElement,
      startOffset,
      endOffset,
      renderedLine.element,
    );
    if (!rectangles) return undefined;
    result.push(...rectangles.map(rectangle => Object.freeze({
      visualLineIndex,
      left: rectangle.left,
      width: rectangle.width,
    })));
  }
  return intersectsRenderedLine ? Object.freeze(result) : undefined;
}

function isCurrentDomTextOffset(element: HTMLElement, offset: number): boolean {
  return Number.isSafeInteger(offset) && offset >= 0 && offset <= element.textContent?.length;
}

export function projectAlphaDecorationOverlays(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[]): void {
  const rectangles = createAlphaVisualDecorationRectangles(context.model, decorations, context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
  const domRectangles = context.useDomTextGeometry
    ? new Map(decorations.map(decoration => [decoration.id, domRangeRectanglesForRange(context, decoration.range)] as const))
    : undefined;
  for (const line of context.renderedLines.values()) reset(line.decorationElement);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of rectangles) {
    if (domRectangles?.get(rectangle.id)) continue;
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-decoration";
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
      const element = ownerDocument.createElement("div");
      element.className = "zeta-alpha-editor-decoration";
      element.classList.add(decoration.presentation);
      element.dataset.decorationId = String(decoration.id);
      if (decoration.hoverText !== undefined) element.title = decoration.hoverText;
      element.style.left = `${rectangle.left}px`;
      element.style.width = `${rectangle.width}px`;
      line.decorationElement.append(element);
    }
  }
  projectDiagnosticGutterMarkers(context, decorations);
}

function projectDiagnosticGutterMarkers(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[]): void {
  const diagnosticsByLine = new Map<number, ResolvedDecoration[]>();
  for (const decoration of decorations) {
    if (!DIAGNOSTIC_PRESENTATION_PRIORITY.has(decoration.presentation)) continue;
    const startLineIndex = decoration.range.start.lineIndex;
    const endLineIndex = decoration.range.end.columnIndex === 0 && decoration.range.end.lineIndex > startLineIndex
      ? decoration.range.end.lineIndex - 1
      : decoration.range.end.lineIndex;
    for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
      const lineDiagnostics = diagnosticsByLine.get(lineIndex) ?? [];
      lineDiagnostics.push(decoration);
      diagnosticsByLine.set(lineIndex, lineDiagnostics);
    }
  }
  for (const [visualLineIndex, line] of context.renderedLines) {
    const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
    const diagnostics = visualLine?.firstForLogicalLine
      ? diagnosticsByLine.get(visualLine.logicalLineIndex) ?? []
      : [];
    const marker = line.diagnosticElement;
    marker.hidden = diagnostics.length === 0;
    marker.classList.remove("error", "warning", "information", "hint");
    delete marker.dataset.diagnosticHoverText;
    marker.removeAttribute("title");
    if (diagnostics.length === 0) continue;
    const highest = diagnostics.reduce((current, candidate) =>
      (DIAGNOSTIC_PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (DIAGNOSTIC_PRESENTATION_PRIORITY.get(current.presentation) ?? 0)
        ? candidate
        : current);
    marker.classList.add(diagnosticMarkerClass(highest.presentation));
    marker.textContent = "●";
    const hoverTexts = [...new Set(diagnostics.flatMap(diagnostic => diagnostic.hoverText === undefined ? [] : [diagnostic.hoverText]))];
    if (hoverTexts.length > 0) {
      const hoverText = hoverTexts.join("\n");
      marker.dataset.diagnosticHoverText = hoverText;
      marker.title = hoverText;
    }
  }
}

function diagnosticMarkerClass(presentation: ResolvedDecoration["presentation"]): "error" | "warning" | "information" | "hint" {
  switch (presentation) {
    case DecorationPresentation.ErrorUnderline: return "error";
    case DecorationPresentation.WarningUnderline: return "warning";
    case DecorationPresentation.InformationUnderline: return "information";
    case DecorationPresentation.HintUnderline: return "hint";
    default: throw new TypeError(`Unknown diagnostic presentation '${presentation}'`);
  }
}

export function projectAlphaCompositionOverlay(context: ViewportOverlayContext, range: TextRange | undefined): void {
  for (const line of context.renderedLines.values()) reset(line.compositionElement);
  if (!range) return;
  const domRectangles = context.useDomTextGeometry ? domRangeRectanglesForRange(context, range) : undefined;
  const rectangles = domRectangles ?? createAlphaVisualRangeRectangles(context.model, [{ range, value: undefined }], context.visualLineProjection, context.renderLines, context.textLeft, context.textMeasurer);
  const ownerDocument = context.ownerDocument;
  for (const rectangle of rectangles) {
    const line = context.renderedLines.get(rectangle.visualLineIndex);
    if (!line) continue;
    const element = ownerDocument.createElement("div");
    element.className = "zeta-alpha-editor-composition";
    element.style.left = `${rectangle.left}px`;
    element.style.width = `${rectangle.width}px`;
    line.compositionElement.append(element);
  }
}
