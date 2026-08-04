import { TextPosition } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { getTextGraphemeBoundaries } from "../../common/core/textSegmentation.js";
import { type EditorScrollPosition } from "../../common/viewLayout/editorViewportModel.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";

export interface AlphaClientPoint {
  readonly clientX: number;
  readonly clientY: number;
}

export interface AlphaViewportPoint {
  readonly left: number;
  readonly top: number;
}

export interface AlphaHitTestLayout {
  readonly lineHeight: number;
  readonly viewportSize: {
    readonly width: number;
    readonly height: number;
  };
  readonly scrollPosition: EditorScrollPosition;
}

export interface AlphaHitTestMetrics {
  readonly gutterWidth: number;
  readonly textLeft: number;
}

export enum AlphaEditorHitTargetKind {
  Gutter = "gutter",
  Text = "text",
  EmptyContent = "emptyContent",
  AfterLines = "afterLines",
}

export interface AlphaEditorHitTarget {
  readonly kind: AlphaEditorHitTargetKind;
  readonly position: TextPosition;
}

/** @internal */
export function hitTestAlphaEditorPoint(
  model: TextModel,
  layout: AlphaHitTestLayout,
  point: AlphaViewportPoint,
  metrics: AlphaHitTestMetrics,
  measurer: AlphaTextMeasurer,
): AlphaEditorHitTarget | undefined {
  validatePoint(point);
  validateLayout(layout);
  validateMetrics(metrics);
  if (
    point.left < 0 ||
    point.top < 0 ||
    point.left >= layout.viewportSize.width ||
    point.top >= layout.viewportSize.height
  ) {
    return undefined;
  }

  const contentTop = point.top + layout.scrollPosition.top;
  const lineIndex = Math.floor(contentTop / layout.lineHeight);
  if (lineIndex >= model.lineCount) {
    const lastLineIndex = model.lineCount - 1;
    return target(
      AlphaEditorHitTargetKind.AfterLines,
      lastLineIndex,
      model.getLineContent(lastLineIndex).length,
    );
  }
  if (point.left < metrics.gutterWidth) {
    return target(AlphaEditorHitTargetKind.Gutter, lineIndex, 0);
  }

  const line = model.getLineContent(lineIndex);
  const textOffset =
    point.left + layout.scrollPosition.left - metrics.textLeft;
  if (textOffset < 0 || line.length === 0) {
    return target(AlphaEditorHitTargetKind.EmptyContent, lineIndex, 0);
  }
  const lineWidth = measurer.measureLineWidth(line);
  if (textOffset >= lineWidth) {
    return target(
      AlphaEditorHitTargetKind.EmptyContent,
      lineIndex,
      line.length,
    );
  }
  return target(
    AlphaEditorHitTargetKind.Text,
    lineIndex,
    nearestCursorColumn(line, textOffset, measurer),
  );
}

/** @internal */
export function hitTestAlphaVisualEditorPoint(model: TextModel, projection: EditorVisualLineProjection, layout: AlphaHitTestLayout, point: AlphaViewportPoint, metrics: AlphaHitTestMetrics, measurer: AlphaTextMeasurer): AlphaEditorHitTarget | undefined {
  validatePoint(point);
  validateLayout(layout);
  validateMetrics(metrics);
  if (projection.modelVersion !== model.version) {
    throw new Error("Alpha visual hit testing requires the current text model projection");
  }
  if (
    point.left < 0 ||
    point.top < 0 ||
    point.left >= layout.viewportSize.width ||
    point.top >= layout.viewportSize.height
  ) {
    return undefined;
  }
  const contentTop = point.top + layout.scrollPosition.top;
  const visualLineIndex = Math.floor(contentTop / layout.lineHeight);
  if (visualLineIndex >= projection.visualLineCount) {
    const logicalLineIndex = model.lineCount - 1;
    return target(AlphaEditorHitTargetKind.AfterLines, logicalLineIndex, model.getLineContent(logicalLineIndex).length);
  }
  const visualLine = projection.lineAt(visualLineIndex)!;
  if (point.left < metrics.gutterWidth) {
    return target(AlphaEditorHitTargetKind.Gutter, visualLine.logicalLineIndex, 0);
  }
  const fullLine = model.getLineContent(visualLine.logicalLineIndex);
  const text = fullLine.slice(visualLine.startColumn, visualLine.endColumn);
  const textOffset = point.left + layout.scrollPosition.left - metrics.textLeft;
  if (textOffset < 0 || text.length === 0) {
    return target(AlphaEditorHitTargetKind.EmptyContent, visualLine.logicalLineIndex, visualLine.startColumn);
  }
  const lineWidth = measurer.measureLineWidth(text);
  if (textOffset >= lineWidth) {
    return target(AlphaEditorHitTargetKind.EmptyContent, visualLine.logicalLineIndex, visualLine.endColumn);
  }
  return target(
    AlphaEditorHitTargetKind.Text,
    visualLine.logicalLineIndex,
    visualLine.startColumn + nearestCursorColumn(text, textOffset, measurer),
  );
}

function nearestCursorColumn(
  line: string,
  horizontalOffset: number,
  measurer: AlphaTextMeasurer,
): number {
  const boundaries = getTextGraphemeBoundaries(line);
  let low = 0;
  let high = boundaries.length - 1;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    const leftColumn = boundaries[middle] ?? 0;
    const rightColumn = boundaries[middle + 1] ?? line.length;
    const left = measurer.measureLineWidth(line.slice(0, leftColumn));
    const right = measurer.measureLineWidth(line.slice(0, rightColumn));
    if (horizontalOffset < left + (right - left) / 2) {
      high = middle;
    } else {
      low = middle + 1;
    }
  }
  return boundaries[low] ?? line.length;
}

function target(
  kind: AlphaEditorHitTargetKind,
  lineIndex: number,
  columnIndex: number,
): AlphaEditorHitTarget {
  return Object.freeze({
    kind,
    position: TextPosition.at(lineIndex, columnIndex),
  });
}

function validatePoint(point: AlphaViewportPoint): void {
  if (
    !point ||
    !Number.isFinite(point.left) ||
    !Number.isFinite(point.top)
  ) {
    throw new RangeError("Alpha hit-test point must contain finite coordinates");
  }
}

function validateLayout(layout: AlphaHitTestLayout): void {
  if (
    !layout ||
    !Number.isFinite(layout.lineHeight) ||
    layout.lineHeight <= 0 ||
    !layout.viewportSize ||
    !Number.isFinite(layout.viewportSize.width) ||
    layout.viewportSize.width < 0 ||
    !Number.isFinite(layout.viewportSize.height) ||
    layout.viewportSize.height < 0 ||
    !layout.scrollPosition ||
    !Number.isFinite(layout.scrollPosition.left) ||
    layout.scrollPosition.left < 0 ||
    !Number.isFinite(layout.scrollPosition.top) ||
    layout.scrollPosition.top < 0
  ) {
    throw new RangeError("Alpha hit-test layout is invalid");
  }
}

function validateMetrics(metrics: AlphaHitTestMetrics): void {
  if (
    !metrics ||
    !Number.isFinite(metrics.gutterWidth) ||
    metrics.gutterWidth < 0 ||
    !Number.isFinite(metrics.textLeft) ||
    metrics.textLeft < metrics.gutterWidth
  ) {
    throw new RangeError("Alpha hit-test metrics are invalid");
  }
}
