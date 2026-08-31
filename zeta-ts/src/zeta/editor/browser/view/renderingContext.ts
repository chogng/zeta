import { type Position } from '../../common/core/position.js';
import { type Range } from '../../common/core/range.js';
import { type TextModel } from '../../common/model/textModel.js';
import { type EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type TextMeasurer } from '../../common/viewModel/textMeasurer.js';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { EditorViewportData } from '../../common/viewLayout/viewLinesViewportData.js';

export interface EditorLineVisibleRange {
	readonly visualLineIndex: number;
	readonly left: number;
	readonly width: number;
}

export interface EditorVisiblePosition {
	readonly visualLineIndex: number;
	readonly left: number;
	readonly isRightToLeft: boolean;
}

export interface EditorOverlayContext {
	readonly ownerDocument: Document;
	readonly model: TextModel;
	readonly visualLineProjection: EditorVisualLineProjection;
	readonly renderLines: EditorViewportLayout['renderLines'];
	readonly textLeft: number;
	readonly textMeasurer: TextMeasurer;
	readonly renderLineHighlight: 'none' | 'gutter' | 'line' | 'all';
	readonly renderLineHighlightOnlyWhenFocus: boolean;
	linesVisibleRangesForRange(range: Range, includeNewLines: boolean): readonly EditorLineVisibleRange[] | undefined;
	visibleRangeForPosition(position: Position): EditorVisiblePosition | undefined;
}

/**
 * Immutable state shared by every Part during one synchronous render pass.
 *
 * The overlay snapshot is omitted when the visual projection is not at the
 * same model version as the layout. Non-overlay Parts can still project the
 * layout while overlay consumers skip stale geometry as one group.
 */
export interface EditorRenderingContext {
	readonly layout: EditorViewportLayout;
	readonly viewportData: EditorViewportData;
	readonly overlay: EditorOverlayContext | undefined;
}

export class FloatHorizontalRange {
	_floatHorizontalRangeBrand: void = undefined;
	public left: number;
	public width: number;

	constructor(left: number, width: number) {
		this.left = left;
		this.width = width;
	}

	public toString(): string {
		return `[${this.left},${this.width}]`;
	}

	public static compare(left: FloatHorizontalRange, right: FloatHorizontalRange): number {
		return left.left - right.left;
	}
}

/** Creates the version-bound context used by one render pass. */
export function createEditorRenderingContext(layout: EditorViewportLayout, overlay: EditorOverlayContext, viewportData = createEditorViewportData(layout, overlay.model.version)): EditorRenderingContext {
	return Object.freeze({
		layout,
		viewportData,
		overlay: overlay.model.version === viewportData.modelVersion && overlay.visualLineProjection.modelVersion === viewportData.modelVersion ? overlay : undefined,
	});
}

/** Adapts the common layout snapshot to the line-rendering viewport contract. */
export function createEditorViewportData(layout: EditorViewportLayout, modelVersion: number): EditorViewportData {
	return new EditorViewportData({
		modelVersion,
		lineHeight: layout.lineHeight,
		visibleLines: layout.visibleLines,
		renderLines: layout.renderLines,
		renderTop: layout.renderTop,
		relativeVerticalOffset: layout.relativeVerticalOffset,
	});
}
