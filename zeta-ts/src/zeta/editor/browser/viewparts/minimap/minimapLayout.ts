import { clamp } from '../../../../base/common/numbers.js';
import type { EditorMinimapLayoutInfo } from '../../../common/config/editorOptions.js';
import type { EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';

export interface MinimapLineSpan {
	readonly top: number;
	readonly height: number;
}

interface MinimapRenderLayoutOptions {
	readonly editorLayout: EditorViewportLayout;
	readonly minimapLayout: EditorMinimapLayoutInfo;
	readonly visualLineCount: number;
	readonly paddingTop: number;
	readonly paddingBottom: number;
}

/** Owns the minimap line window, slider geometry, markers, and pointer coordinate mapping for one render. */
export class MinimapRenderLayout {
	public readonly key: string;

	private constructor(
		public readonly startVisualLineIndex: number,
		public readonly endVisualLineIndexExclusive: number,
		public readonly topPaddingInnerHeight: number,
		public readonly bottomPaddingInnerHeight: number,
		public readonly sliderNeeded: boolean,
		public readonly sliderTop: number,
		public readonly sliderHeight: number,
		private readonly visualLineCount: number,
		private readonly isSampling: boolean,
		private readonly pixelRatio: number,
		private readonly minimapLineHeight: number,
		private readonly canvasOuterHeight: number,
		private readonly editorLineHeight: number,
		private readonly editorViewportHeight: number,
		private readonly maximumScrollTop: number,
		private readonly maximumSliderTop: number,
		private readonly paddingTop: number,
	) {
		this.key = [
			startVisualLineIndex,
			endVisualLineIndexExclusive,
			topPaddingInnerHeight,
			bottomPaddingInnerHeight,
			isSampling ? 1 : 0,
			pixelRatio,
		].join(':');
		Object.freeze(this);
	}

	public static create(options: MinimapRenderLayoutOptions): MinimapRenderLayout {
		const editorLayout = options.editorLayout;
		const minimapLayout = options.minimapLayout;
		const visualLineCount = Math.max(1, options.visualLineCount);
		const pixelRatio = readPixelRatio(minimapLayout);
		const canvasOuterHeight = Math.max(0, minimapLayout.minimapCanvasOuterHeight);
		const minimapLineHeight = Math.max(1, minimapLayout.minimapLineHeight);
		const rowHeight = minimapLineHeight / pixelRatio;
		const rowCapacity = Math.max(1, Math.floor(minimapLayout.minimapCanvasInnerHeight / minimapLineHeight));
		const editorLineHeight = Math.max(1, editorLayout.lineHeight);
		const editorViewportHeight = Math.max(0, editorLayout.viewportSize.height);
		const maximumScrollTop = Math.max(0, editorLayout.maximumScrollPosition.top);
		const scrollProgress = maximumScrollTop > 0 ? clamp(editorLayout.scrollPosition.top / maximumScrollTop, 0, 1) : 0;
		const extraRowsAtTop = Math.floor(Math.max(0, options.paddingTop) / editorLineHeight);
		const extraRowsAtBottom = Math.floor(Math.max(0, options.paddingBottom) / editorLineHeight);

		let sliderHeight: number;
		let maximumSliderTop: number;
		if (minimapLayout.minimapHeightIsEditorHeight) {
			sliderHeight = Math.max(2, Math.min(canvasOuterHeight, canvasOuterHeight * editorViewportHeight / Math.max(1, editorLayout.contentSize.height)));
			maximumSliderTop = Math.max(0, canvasOuterHeight - sliderHeight);
		} else {
			sliderHeight = Math.max(2, Math.min(canvasOuterHeight, editorViewportHeight / editorLineHeight * rowHeight));
			const documentHeight = (extraRowsAtTop + visualLineCount + extraRowsAtBottom) * rowHeight;
			maximumSliderTop = Math.min(
				Math.max(0, canvasOuterHeight - sliderHeight),
				Math.max(0, documentHeight - sliderHeight),
			);
		}
		const desiredSliderTop = scrollProgress * maximumSliderTop;
		const sliderNeeded = maximumScrollTop > 0 && maximumSliderTop > 0;

		if (minimapLayout.minimapHeightIsEditorHeight || extraRowsAtTop + visualLineCount + extraRowsAtBottom <= rowCapacity) {
			return new MinimapRenderLayout(
				0,
				visualLineCount,
				extraRowsAtTop * minimapLineHeight,
				extraRowsAtBottom * minimapLineHeight,
				sliderNeeded,
				desiredSliderTop,
				sliderHeight,
				visualLineCount,
				minimapLayout.minimapIsSampling,
				pixelRatio,
				minimapLineHeight,
				canvasOuterHeight,
				editorLineHeight,
				editorViewportHeight,
				maximumScrollTop,
				maximumSliderTop,
				options.paddingTop,
			);
		}

		const visibleLineOffset = clamp((editorLayout.scrollPosition.top - options.paddingTop) / editorLineHeight, 0, visualLineCount);
		const maximumStartVisualLineIndex = Math.max(0, visualLineCount - rowCapacity);
		const startVisualLineIndex = scrollProgress === 1
			? maximumStartVisualLineIndex
			: clamp(Math.round(visibleLineOffset - desiredSliderTop / rowHeight), 0, maximumStartVisualLineIndex);
		const endVisualLineIndexExclusive = Math.min(visualLineCount, startVisualLineIndex + rowCapacity);
		const alignedSliderTop = clamp(
			(visibleLineOffset - startVisualLineIndex) * rowHeight,
			0,
			Math.max(0, canvasOuterHeight - sliderHeight),
		);
		return new MinimapRenderLayout(
			startVisualLineIndex,
			endVisualLineIndexExclusive,
			0,
			0,
			sliderNeeded,
			alignedSliderTop,
			sliderHeight,
			visualLineCount,
			false,
			pixelRatio,
			minimapLineHeight,
			canvasOuterHeight,
			editorLineHeight,
			editorViewportHeight,
			maximumScrollTop,
			maximumSliderTop,
			options.paddingTop,
		);
	}

	public lineSpan(startVisualLineIndex: number, endVisualLineIndexExclusive: number): MinimapLineSpan | undefined {
		let top: number;
		let bottom: number;
		if (this.isSampling) {
			const sampleTop = this.topPaddingInnerHeight / this.pixelRatio;
			const sampleHeight = Math.max(0, this.canvasOuterHeight - sampleTop - this.bottomPaddingInnerHeight / this.pixelRatio);
			top = sampleTop + startVisualLineIndex / this.visualLineCount * sampleHeight;
			bottom = sampleTop + endVisualLineIndexExclusive / this.visualLineCount * sampleHeight;
		} else {
			top = (this.topPaddingInnerHeight + (startVisualLineIndex - this.startVisualLineIndex) * this.minimapLineHeight) / this.pixelRatio;
			bottom = (this.topPaddingInnerHeight + (endVisualLineIndexExclusive - this.startVisualLineIndex) * this.minimapLineHeight) / this.pixelRatio;
		}
		const clippedTop = clamp(top, 0, this.canvasOuterHeight);
		const clippedBottom = clamp(bottom, 0, this.canvasOuterHeight);
		if (clippedBottom <= clippedTop) return undefined;
		return Object.freeze({ top: clippedTop, height: clippedBottom - clippedTop });
	}

	public scrollTopAt(canvasOffset: number): number {
		if (this.maximumScrollTop === 0 || this.canvasOuterHeight === 0) return 0;
		let visualLineOffset: number;
		if (this.isSampling) {
			const sampleTop = this.topPaddingInnerHeight / this.pixelRatio;
			const sampleHeight = Math.max(1, this.canvasOuterHeight - sampleTop - this.bottomPaddingInnerHeight / this.pixelRatio);
			visualLineOffset = clamp((canvasOffset - sampleTop) / sampleHeight, 0, 1) * this.visualLineCount;
		} else {
			visualLineOffset = this.startVisualLineIndex + (canvasOffset * this.pixelRatio - this.topPaddingInnerHeight) / this.minimapLineHeight;
		}
		const centeredScrollTop = this.paddingTop + (clamp(visualLineOffset, 0, this.visualLineCount) + 0.5) * this.editorLineHeight - this.editorViewportHeight / 2;
		return clamp(centeredScrollTop, 0, this.maximumScrollTop);
	}

	public scrollTopAtSliderPosition(canvasOffset: number, pointerSliderOffset: number): number {
		if (this.maximumScrollTop === 0 || this.maximumSliderTop === 0) return 0;
		const sliderTop = clamp(canvasOffset - pointerSliderOffset, 0, this.maximumSliderTop);
		return sliderTop / this.maximumSliderTop * this.maximumScrollTop;
	}
}

function readPixelRatio(layout: EditorMinimapLayoutInfo): number {
	if (layout.minimapCanvasOuterHeight <= 0) return 1;
	return Math.max(1, layout.minimapCanvasInnerHeight / layout.minimapCanvasOuterHeight);
}
