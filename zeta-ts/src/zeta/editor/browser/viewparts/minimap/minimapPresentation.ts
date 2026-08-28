export const MINIMAP_WIDTH = 56;
export const MINIMAP_LINE_HEIGHT = 2;
export const MINIMAP_MINIMUM_EDITOR_WIDTH = 240;

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

interface MinimapSliderLayout {
	readonly visible: boolean;
	readonly height: number;
	readonly top: number;
}

interface MinimapVerticalLayout {
	readonly lineScale: number;
	readonly slider: MinimapSliderLayout;
}

/** Keeps short documents at the top and compresses only documents taller than the minimap. */
export function createMinimapVerticalLayout(viewportHeight: number, contentHeight: number, scrollTop: number, lineHeight: number, lineCount: number): MinimapVerticalLayout {
	const naturalContentHeight = lineCount * MINIMAP_LINE_HEIGHT;
	const minimapContentHeight = Math.min(viewportHeight, naturalContentHeight);
	const lineScale = lineCount > 0 ? minimapContentHeight / lineCount : 0;
	const maximumScrollTop = Math.max(0, contentHeight - viewportHeight);
	if (maximumScrollTop === 0 || minimapContentHeight === 0) {
		return Object.freeze({
			lineScale,
			slider: Object.freeze({ visible: false, height: 0, top: 0 }),
		});
	}
	const visibleLineCount = viewportHeight / lineHeight;
	const sliderHeight = Math.min(minimapContentHeight, Math.max(2, visibleLineCount * lineScale));
	const sliderTrackSize = Math.max(0, minimapContentHeight - sliderHeight);
	return Object.freeze({
		lineScale,
		slider: Object.freeze({
			visible: sliderTrackSize > 0,
			height: sliderHeight,
			top: scrollTop / maximumScrollTop * sliderTrackSize,
		}),
	});
}
