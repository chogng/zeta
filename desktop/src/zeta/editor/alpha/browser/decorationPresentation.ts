import { type Event } from "../../../base/common/event.js";
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from "../common/decoration.js";
import { type TextRange } from "../common/text.js";
import { type TextModel } from "../common/textModel.js";
import { type EditorVisualLineProjection } from "../common/visualLineProjection.js";
import { type EditorLineRange } from "../common/viewport.js";
import { type AlphaTextMeasurer } from "./fontMetrics.js";
import { AlphaEmptyRangeRendering, createAlphaRangeRectangles } from "./rangeGeometry.js";
import { createAlphaVisualRangeRectangles } from "./visualRangeGeometry.js";

export enum AlphaDecorationPresentation {
  SearchMatch = "search-match",
  OccurrenceHighlight = "occurrence-highlight",
  BracketMatch = "bracket-match",
  ErrorUnderline = "error-underline",
  WarningUnderline = "warning-underline",
  InformationUnderline = "information-underline",
  HintUnderline = "hint-underline",
}

export interface AlphaResolvedDecoration {
  readonly id: TextDecorationId;
  readonly range: TextRange;
  readonly presentation: AlphaDecorationPresentation;
  readonly hoverText?: string;
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
  readonly hoverText?: string;
}

export interface AlphaVisualDecorationRectangle {
  readonly id: TextDecorationId;
  readonly presentation: AlphaDecorationPresentation;
  readonly visualLineIndex: number;
  readonly left: number;
  readonly width: number;
  readonly hoverText?: string;
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
  resolveHoverText?: (decoration: TextDecorationSnapshot<TMetadata>) => string | undefined,
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
        const hoverText = resolveHoverText?.(decoration);
        if (hoverText !== undefined && (typeof hoverText !== "string" || hoverText.trim().length === 0)) {
          throw new TypeError("Alpha decoration hover text must be non-empty text");
        }
        resolved.push(Object.freeze({
          id: decoration.id,
          range: decoration.range,
          presentation,
          ...(hoverText === undefined ? {} : { hoverText }),
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
    AlphaEmptyRangeRendering.RenderAsSpace,
  ).map(rectangle => Object.freeze({
    id: rectangle.value.id,
    presentation: rectangle.value.presentation,
    lineIndex: rectangle.lineIndex,
    left: rectangle.left,
    width: rectangle.width,
    ...(rectangle.value.hoverText === undefined ? {} : { hoverText: rectangle.value.hoverText }),
  })));
}

/** @internal */
export function createAlphaVisualDecorationRectangles(model: TextModel, decorations: readonly AlphaResolvedDecoration[], projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: AlphaTextMeasurer): readonly AlphaVisualDecorationRectangle[] {
  return Object.freeze(createAlphaVisualRangeRectangles(
    model,
    decorations.map(decoration => ({
      range: decoration.range,
      value: decoration,
    })),
    projection,
    renderLines,
    textLeft,
    measurer,
    AlphaEmptyRangeRendering.RenderAsSpace,
  ).map(rectangle => Object.freeze({
    id: rectangle.value.id,
    presentation: rectangle.value.presentation,
    visualLineIndex: rectangle.visualLineIndex,
    left: rectangle.left,
    width: rectangle.width,
    ...(rectangle.value.hoverText === undefined ? {} : { hoverText: rectangle.value.hoverText }),
  })));
}

function validatePresentation(
  presentation: AlphaDecorationPresentation,
): void {
  if (
    presentation !== AlphaDecorationPresentation.SearchMatch &&
    presentation !== AlphaDecorationPresentation.OccurrenceHighlight &&
    presentation !== AlphaDecorationPresentation.BracketMatch &&
    presentation !== AlphaDecorationPresentation.ErrorUnderline &&
    presentation !== AlphaDecorationPresentation.WarningUnderline &&
    presentation !== AlphaDecorationPresentation.InformationUnderline &&
    presentation !== AlphaDecorationPresentation.HintUnderline
  ) {
    throw new TypeError(`Unknown Alpha decoration presentation '${presentation}'`);
  }
}
