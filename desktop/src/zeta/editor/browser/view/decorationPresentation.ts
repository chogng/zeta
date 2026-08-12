import { type Event } from "../../../base/common/event.js";
import { type TextDecorationCollection, type TextDecorationId, type TextDecorationSnapshot } from "../../common/model/decorationCollection.js";
import { type TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange } from "../../common/viewLayout/editorViewportModel.js";
import { type TextMeasurer } from "./fontMetrics.js";
import { EmptyRangeRendering, createAlphaRangeRectangles } from "./rangeGeometry.js";
import { createAlphaVisualRangeRectangles } from "./visualRangeGeometry.js";

export enum DecorationPresentation {
  SearchMatch = "search-match",
  OccurrenceHighlight = "occurrence-highlight",
  BracketMatch = "bracket-match",
  ErrorUnderline = "error-underline",
  WarningUnderline = "warning-underline",
  InformationUnderline = "information-underline",
  HintUnderline = "hint-underline",
  UnicodeHighlight = "unicode-highlight",
  UnusualLineTerminator = "unusual-line-terminator",
}

export interface ResolvedDecoration {
  readonly id: TextDecorationId;
  readonly range: TextRange;
  readonly presentation: DecorationPresentation;
  readonly hoverText?: string;
}

export interface DecorationSource {
  readonly onDidChange: Event<void>;
  readonly decorations: readonly ResolvedDecoration[];
}

export interface DecorationRectangle {
  readonly id: TextDecorationId;
  readonly presentation: DecorationPresentation;
  readonly lineIndex: number;
  readonly left: number;
  readonly width: number;
  readonly hoverText?: string;
}

export interface VisualDecorationRectangle {
  readonly id: TextDecorationId;
  readonly presentation: DecorationPresentation;
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
  ) => DecorationPresentation | undefined,
  resolveHoverText?: (decoration: TextDecorationSnapshot<TMetadata>) => string | undefined,
): DecorationSource {
  const onDidChange: Event<void> = listener => {
    return collection.onDidChange(() => listener());
  };
  return Object.freeze({
    onDidChange,
    get decorations(): readonly ResolvedDecoration[] {
      const resolved: ResolvedDecoration[] = [];
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
  decorations: readonly ResolvedDecoration[],
  renderLines: EditorLineRange,
  textLeft: number,
  measurer: TextMeasurer,
): readonly DecorationRectangle[] {
  return Object.freeze(createAlphaRangeRectangles(
    model,
    decorations.map(decoration => ({
      range: decoration.range,
      value: decoration,
    })),
    renderLines,
    textLeft,
    measurer,
    EmptyRangeRendering.RenderAsSpace,
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
export function createAlphaVisualDecorationRectangles(model: TextModel, decorations: readonly ResolvedDecoration[], projection: EditorVisualLineProjection, renderLines: EditorLineRange, textLeft: number, measurer: TextMeasurer): readonly VisualDecorationRectangle[] {
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
    EmptyRangeRendering.RenderAsSpace,
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
  presentation: DecorationPresentation,
): void {
  if (
    presentation !== DecorationPresentation.SearchMatch &&
    presentation !== DecorationPresentation.OccurrenceHighlight &&
    presentation !== DecorationPresentation.BracketMatch &&
    presentation !== DecorationPresentation.ErrorUnderline &&
    presentation !== DecorationPresentation.WarningUnderline &&
    presentation !== DecorationPresentation.InformationUnderline &&
    presentation !== DecorationPresentation.HintUnderline
    && presentation !== DecorationPresentation.UnicodeHighlight
    && presentation !== DecorationPresentation.UnusualLineTerminator
  ) {
    throw new TypeError(`Unknown Alpha decoration presentation '${presentation}'`);
  }
}
