import { type TextSelectionSet } from "../../common/core/selection.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange } from "../../common/viewLayout/editorViewportModel.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { createAlphaVisualRangeRectangles } from "./visualRangeGeometry.js";

export interface AlphaVisualSelectionRectangle {
  readonly selectionIndex: number;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly width: number;
}

export interface AlphaVisualCaretRectangle {
  readonly selectionIndex: number;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly primary: boolean;
}

export interface AlphaVisualSelectionGeometry {
  readonly selections: readonly AlphaVisualSelectionRectangle[];
  readonly carets: readonly AlphaVisualCaretRectangle[];
}

/** @internal */
export function createAlphaVisualSelectionGeometry(model: TextModel, selectionSet: TextSelectionSet, projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: AlphaTextMeasurer): AlphaVisualSelectionGeometry {
  if (projection.modelVersion !== model.version) {
    throw new Error("Visual selection geometry requires the current text model projection");
  }
  const carets: AlphaVisualCaretRectangle[] = [];
  for (let selectionIndex = 0; selectionIndex < selectionSet.selections.length; selectionIndex += 1) {
    const selection = selectionSet.selections[selectionIndex];
    if (!selection) continue;
    const visualLineIndex = projection.visualLineIndexAt(selection.active);
    if (visualLineIndex < renderLines.startLineIndex || visualLineIndex >= renderLines.endLineIndexExclusive) continue;
    const visualLine = projection.lineAt(visualLineIndex)!;
    const text = model.getLineContent(visualLine.logicalLineIndex);
    carets.push(Object.freeze({
      selectionIndex,
      visualLineIndex,
      left: textLeft + measurer.measureLineWidth(text.slice(visualLine.startColumn, selection.active.columnIndex)),
      primary: selectionIndex === selectionSet.primaryIndex,
    }));
  }
  const selections = createAlphaVisualRangeRectangles(
    model,
    selectionSet.selections.map((selection, selectionIndex) => ({
      range: selection.range,
      value: selectionIndex,
    })),
    projection,
    renderLines,
    textLeft,
    measurer,
  ).map(rectangle => Object.freeze({
    selectionIndex: rectangle.value,
    visualLineIndex: rectangle.visualLineIndex,
    left: rectangle.left,
    width: rectangle.width,
  }));
  return Object.freeze({
    selections: Object.freeze(selections),
    carets: Object.freeze(carets),
  });
}
