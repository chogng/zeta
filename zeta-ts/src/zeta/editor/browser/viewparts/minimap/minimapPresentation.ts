import { createScrollbarAxisMetrics } from "../../../../base/browser/ui/scrollbar/scrollbarState.js";

export const MINIMAP_WIDTH = 56;
export const MINIMAP_LINE_HEIGHT = 1;

const MINIMAP_CONTENT_LEFT_INSET = 8;
const MINIMAP_CONTENT_RIGHT_INSET = 4;
const MINIMAP_MINIMUM_CONTENT_WIDTH = 4;

/** Maps normalized document density into the minimap's inset content lane. */
export function minimapContentWidth(density: number, minimapWidth = MINIMAP_WIDTH): number {
  const availableWidth = Math.max(0, minimapWidth - MINIMAP_CONTENT_LEFT_INSET - MINIMAP_CONTENT_RIGHT_INSET);
  return Math.min(availableWidth, Math.max(MINIMAP_MINIMUM_CONTENT_WIDTH, density * availableWidth));
}

/** Right inset shared by the DOM and GPU minimap projections. */
export const MINIMAP_CONTENT_RIGHT = MINIMAP_CONTENT_RIGHT_INSET;

export interface MinimapSliderLayout {
  readonly height: number;
  readonly top: number;
}

/** Mirrors the canonical vertical scrollbar thumb over the minimap track. */
export function createMinimapSliderLayout(viewportHeight: number, contentHeight: number, scrollTop: number): MinimapSliderLayout {
  const metrics = createScrollbarAxisMetrics(viewportHeight, contentHeight, scrollTop, viewportHeight, 2);
  return Object.freeze({ height: metrics.thumbSize, top: metrics.thumbPosition });
}
