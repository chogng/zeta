import { type TextModel } from "../../../common/model/textModel.js";

export interface StickyScrollRegion { readonly startLineIndex: number; readonly endLineIndex: number; readonly label?: string; }
export interface StickyScrollEntry { readonly lineIndex: number; readonly label: string; readonly depth: number; }

/** Calculates the visible ancestor headers that a sticky-scroll renderer should project. */
export function buildStickyScrollEntries(model: TextModel, firstVisibleLineIndex: number, regions: readonly StickyScrollRegion[], maxEntries = 5): readonly StickyScrollEntry[] {
  if (!Number.isSafeInteger(firstVisibleLineIndex) || firstVisibleLineIndex < 0 || firstVisibleLineIndex >= model.lineCount) throw new RangeError("Sticky scroll visible line is outside the text model");
  const active = regions.filter(region => region.startLineIndex < firstVisibleLineIndex && region.endLineIndex >= firstVisibleLineIndex).sort((left, right) => left.startLineIndex - right.startLineIndex || right.endLineIndex - left.endLineIndex);
  const selected = active.slice(Math.max(0, active.length - maxEntries));
  return Object.freeze(selected.map((region, index) => Object.freeze({ lineIndex: region.startLineIndex, label: region.label ?? model.getLineContent(region.startLineIndex).trim(), depth: index })));
}
