import { type TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange } from "../../common/viewLayout/editorViewportModel.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { AlphaEmptyRangeRendering } from "./rangeGeometry.js";

export interface AlphaVisualRangeGeometryEntry<T> {
  readonly range: TextRange;
  readonly value: T;
}

export interface AlphaVisualRangeRectangle<T> {
  readonly value: T;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly width: number;
}

/** @internal */
export function createAlphaVisualRangeRectangles<T>(model: TextModel, entries: readonly AlphaVisualRangeGeometryEntry<T>[], projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: AlphaTextMeasurer, emptyRangeRendering = AlphaEmptyRangeRendering.Ignore): readonly AlphaVisualRangeRectangle<T>[] {
  if (projection.modelVersion !== model.version) {
    throw new Error("Visual range geometry requires the current text model projection");
  }
  const rectangles: AlphaVisualRangeRectangle<T>[] = [];
  const newlineWidth = measurer.measureLineWidth(" ");
  for (const entry of entries) {
    if (entry.range.empty && emptyRangeRendering === AlphaEmptyRangeRendering.RenderAsSpace) {
      const visualLineIndex = projection.visualLineIndexAt(entry.range.start);
      const visualLine = projection.lineAt(visualLineIndex);
      if (!visualLine || visualLineIndex < renderLines.startLineIndex || visualLineIndex >= renderLines.endLineIndexExclusive) continue;
      const logicalText = model.getLineContent(visualLine.logicalLineIndex);
      rectangles.push(Object.freeze({
        value: entry.value,
        visualLineIndex,
        left: textLeft + measurer.measureLineWidth(logicalText.slice(visualLine.startColumn, entry.range.start.columnIndex)),
        width: Math.max(1, newlineWidth),
      }));
      continue;
    }
    if (entry.range.empty) continue;
    for (let visualLineIndex = renderLines.startLineIndex; visualLineIndex < renderLines.endLineIndexExclusive; visualLineIndex += 1) {
      const visualLine = projection.lineAt(visualLineIndex);
      if (!visualLine || !intersectsLogicalLine(entry.range, visualLine.logicalLineIndex)) continue;
      const logicalText = model.getLineContent(visualLine.logicalLineIndex);
      const startsOnLogicalLine = visualLine.logicalLineIndex === entry.range.start.lineIndex;
      const endsOnLogicalLine = visualLine.logicalLineIndex === entry.range.end.lineIndex;
      const startColumn = startsOnLogicalLine
        ? Math.max(visualLine.startColumn, entry.range.start.columnIndex)
        : visualLine.startColumn;
      const endColumn = endsOnLogicalLine
        ? Math.min(visualLine.endColumn, entry.range.end.columnIndex)
        : visualLine.endColumn;
      if (endColumn < startColumn) continue;
      if (endsOnLogicalLine && endColumn === 0 && !startsOnLogicalLine) continue;
      const left = textLeft + measurer.measureLineWidth(logicalText.slice(visualLine.startColumn, startColumn));
      let right = textLeft + measurer.measureLineWidth(logicalText.slice(visualLine.startColumn, endColumn));
      if (!endsOnLogicalLine && visualLine.lastForLogicalLine) right += newlineWidth;
      if (right <= left) continue;
      rectangles.push(Object.freeze({
        value: entry.value,
        visualLineIndex,
        left,
        width: right - left,
      }));
    }
  }
  return Object.freeze(rectangles);
}

function intersectsLogicalLine(range: TextRange, lineIndex: number): boolean {
  return lineIndex >= range.start.lineIndex && lineIndex <= range.end.lineIndex;
}
