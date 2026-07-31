import { type TextSelectionSet } from "../common/selection.js";
import { type TextModel } from "../common/textModel.js";
import { type EditorLineRange } from "../common/viewport.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { createAlphaRangeRectangles } from "./rangeGeometry.js";

export interface AlphaSelectionRectangle {
  readonly selectionIndex: number;
  readonly lineIndex: number;
  readonly left: number;
  readonly width: number;
}

export interface AlphaCaretRectangle {
  readonly selectionIndex: number;
  readonly lineIndex: number;
  readonly left: number;
  readonly primary: boolean;
}

export interface AlphaSelectionGeometry {
  readonly selections: readonly AlphaSelectionRectangle[];
  readonly carets: readonly AlphaCaretRectangle[];
}

/** @internal */
export function createAlphaSelectionGeometry(
  model: TextModel,
  selectionSet: TextSelectionSet,
  renderLines: EditorLineRange,
  textLeft: number,
  measurer: AlphaTextMeasurer,
): AlphaSelectionGeometry {
  const carets: AlphaCaretRectangle[] = [];

  for (
    let selectionIndex = 0;
    selectionIndex < selectionSet.selections.length;
    selectionIndex++
  ) {
    const selection = selectionSet.selections[selectionIndex];
    if (!selection) continue;
    if (containsLine(renderLines, selection.active.lineIndex)) {
      carets.push(Object.freeze({
        selectionIndex,
        lineIndex: selection.active.lineIndex,
        left: textLeft + prefixWidth(
          model,
          selection.active.lineIndex,
          selection.active.columnIndex,
          measurer,
        ),
        primary: selectionIndex === selectionSet.primaryIndex,
      }));
    }
  }
  const selections = createAlphaRangeRectangles(
    model,
    selectionSet.selections.map((selection, selectionIndex) => ({
      range: selection.range,
      value: selectionIndex,
    })),
    renderLines,
    textLeft,
    measurer,
  ).map(rectangle => Object.freeze({
    selectionIndex: rectangle.value,
    lineIndex: rectangle.lineIndex,
    left: rectangle.left,
    width: rectangle.width,
  }));

  return Object.freeze({
    selections: Object.freeze(selections),
    carets: Object.freeze(carets),
  });
}

function prefixWidth(
  model: TextModel,
  lineIndex: number,
  columnIndex: number,
  measurer: AlphaTextMeasurer,
): number {
  return measurer.measureLineWidth(
    model.getLineContent(lineIndex).slice(0, columnIndex),
  );
}

function containsLine(range: EditorLineRange, lineIndex: number): boolean {
  return lineIndex >= range.startLineIndex &&
    lineIndex < range.endLineIndexExclusive;
}
