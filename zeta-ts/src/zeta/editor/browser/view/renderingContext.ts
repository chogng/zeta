import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { ViewportData } from '../../common/viewLayout/viewLinesViewportData.js';
import { type ViewportOverlayContext } from '../viewparts/viewportOverlay/viewportOverlayPresentation.js';

/**
 * Immutable state shared by every Part during one synchronous render pass.
 *
 * The overlay snapshot is omitted when the visual projection is not at the
 * same model version as the layout. Non-overlay Parts can still project the
 * layout while overlay consumers skip stale geometry as one group.
 */
export interface EditorRenderingContext {
	readonly layout: EditorViewportLayout;
	readonly viewportData: ViewportData;
	readonly overlay: ViewportOverlayContext | undefined;
}

export class FloatHorizontalRange {
	constructor(
		public left: number,
		public width: number,
	) {}

	public static compare(left: FloatHorizontalRange, right: FloatHorizontalRange): number {
		return left.left - right.left;
	}
}

/** Creates the version-bound context used by one render pass. */
export function createEditorRenderingContext(layout: EditorViewportLayout, overlay: ViewportOverlayContext, viewportData = createEditorViewportData(layout)): EditorRenderingContext {
	return Object.freeze({
		layout,
		viewportData,
		overlay: overlay.model.version === layout.modelVersion && overlay.visualLineProjection.modelVersion === layout.modelVersion ? overlay : undefined,
	});
}

/** Adapts the common layout snapshot to the line-rendering viewport contract. */
export function createEditorViewportData(layout: EditorViewportLayout): ViewportData {
	return new ViewportData({
		modelVersion: layout.modelVersion,
		lineHeight: layout.lineHeight,
		visibleLines: layout.visibleLines,
		renderLines: layout.renderLines,
		renderTop: layout.renderTop,
	});
}
