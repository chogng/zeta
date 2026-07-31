import { type Event } from "../../../base/common/event.js";
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from "../common/decoration.js";
import { type TextRange } from "../common/text.js";
import { type TextModel } from "../common/textModel.js";
import { type EditorLineRange } from "../common/viewport.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { createAlphaRangeRectangles } from "./rangeGeometry.js";

export enum AlphaDecorationPresentation {
  SearchMatch = "search-match",
  ErrorUnderline = "error-underline",
  WarningUnderline = "warning-underline",
}

export interface AlphaResolvedDecoration {
  readonly id: TextDecorationId;
  readonly range: TextRange;
  readonly presentation: AlphaDecorationPresentation;
}

export interface AlphaDecorationSource {
  readonly onDidChange: Event<void>;
  readonly decorations: readonly AlphaResolvedDecoration[];
}

export interface AlphaDecorationRectangle {
  readonly id: TextDecorationId;
  readonly presentation: AlphaDecorationPresentation;
  readonly lineIndex: number;
  readonly left: number;
  readonly width: number;
}

/**
 * Adapts one caller-owned decoration collection to browser presentation.
 *
 * The adapter and its consumers observe the collection but do not own it.
 * Returning `undefined` from `resolvePresentation` omits a decoration from
 * this renderer without changing the common collection.
 */
export function createAlphaDecorationSource<TMetadata>(
  collection: TextDecorationCollection<TMetadata>,
  resolvePresentation: (
    decoration: TextDecorationSnapshot<TMetadata>,
  ) => AlphaDecorationPresentation | undefined,
): AlphaDecorationSource {
  const onDidChange: Event<void> = listener => {
    return collection.onDidChange(() => listener());
  };
  return Object.freeze({
    onDidChange,
    get decorations(): readonly AlphaResolvedDecoration[] {
      const resolved: AlphaResolvedDecoration[] = [];
      for (const decoration of collection.decorations) {
        const presentation = resolvePresentation(decoration);
        if (presentation === undefined) continue;
        validatePresentation(presentation);
        resolved.push(Object.freeze({
          id: decoration.id,
          range: decoration.range,
          presentation,
        }));
      }
      return Object.freeze(resolved);
    },
  });
}

/** @internal */
export function createAlphaDecorationRectangles(
  model: TextModel,
  decorations: readonly AlphaResolvedDecoration[],
  renderLines: EditorLineRange,
  textLeft: number,
  measurer: AlphaTextMeasurer,
): readonly AlphaDecorationRectangle[] {
  return Object.freeze(createAlphaRangeRectangles(
    model,
    decorations.map(decoration => ({
      range: decoration.range,
      value: decoration,
    })),
    renderLines,
    textLeft,
    measurer,
  ).map(rectangle => Object.freeze({
    id: rectangle.value.id,
    presentation: rectangle.value.presentation,
    lineIndex: rectangle.lineIndex,
    left: rectangle.left,
    width: rectangle.width,
  })));
}

function validatePresentation(
  presentation: AlphaDecorationPresentation,
): void {
  if (
    presentation !== AlphaDecorationPresentation.SearchMatch &&
    presentation !== AlphaDecorationPresentation.ErrorUnderline &&
    presentation !== AlphaDecorationPresentation.WarningUnderline
  ) {
    throw new TypeError(`Unknown Alpha decoration presentation '${presentation}'`);
  }
}
