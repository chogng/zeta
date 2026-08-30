import './minimap.css';
import { addDisposableListener, fragment, h, reset } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { RunOnceScheduler } from '../../../../base/common/async.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { clamp } from '../../../../base/common/numbers.js';
import { RGBA8 } from '../../../common/core/misc/rgba.js';
import { RenderMinimap, type EditorMinimapLayoutInfo, type EditorMinimapOptions } from '../../../common/config/editorOptions.js';
import type { TextModel } from '../../../common/model/textModel.js';
import type { ResolvedSemanticToken, SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';
import type { EditorScrollPosition } from '../../../common/viewModel/editorViewportContracts.js';
import type { EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import type { EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';
import { EditorViewPart, type EditorRenderingContext } from '../../view/viewPart.js';
import type { DiagnosticOverviewMarker } from '../overviewRuler/overviewRuler.js';
import type { DiffOverviewMarker } from '../overviewRuler/overviewRuler.js';
import { MinimapCharRendererFactory } from './minimapCharRendererFactory.js';

type MinimapMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

interface MinimapOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly options: EditorMinimapOptions;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly tabSize: number;
	readonly paddingTop: number;
	readonly paddingBottom: number;
	readonly readLayout: () => EditorViewportLayout;
	readonly readMinimapLayout: () => EditorMinimapLayoutInfo;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly readMarkers: () => readonly MinimapMarker[];
	readonly readMarkersRevision: () => number;
	readonly scrollTo: (position: EditorScrollPosition) => void;
}

/** Owns the minimap canvas, marker layer, slider, and pointer navigation. */
export class EditorMinimap extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly canvas: HTMLCanvasElement;
	private readonly markerLayer: HTMLDivElement;
	private readonly slider: HTMLDivElement;
	private readonly options: MinimapOptions;
	private pointerId: number | undefined;
	private pointerSliderOffset: number | undefined;
	private renderedCanvasKey = '';
	private renderedMarkersKey = '';
	private semanticTokensRevision = 0;
	private lastScrollTop = 0;
	private readonly scrollAutohideScheduler: RunOnceScheduler;

	constructor(options: MinimapOptions) {
		super();
		this.options = options;
		const ownerDocument = options.host.ownerDocument;
		this.domNode = h(ownerDocument, 'div');
		this.root = new FastDomNode(this.domNode);
		this.canvas = h(ownerDocument, 'canvas');
		this.markerLayer = h(ownerDocument, 'div');
		this.slider = h(ownerDocument, 'div');
		this.root.setClassName('stanza-editor-minimap');
		this.canvas.className = 'stanza-editor-minimap-canvas';
		this.markerLayer.className = 'stanza-editor-minimap-markers';
		this.slider.className = 'stanza-editor-minimap-slider';
		this.domNode.classList.toggle('show-slider-always', options.options.showSlider === 'always');
		this.domNode.classList.toggle('side-left', options.options.side === 'left');
		this.domNode.classList.add(`autohide-${options.options.autohide}`);
		this.domNode.setAttribute('aria-hidden', 'true');
		this.domNode.append(this.canvas, this.markerLayer, this.slider);
		this._register(toDisposable(() => this.domNode.remove()));
		this._register(addDisposableListener<PointerEvent>(this.domNode, 'pointerdown', event => this.handlePointerDown(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, 'pointermove', event => this.handlePointerMove(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, 'pointerup', event => this.handlePointerEnd(event)));
		this._register(addDisposableListener<PointerEvent>(ownerDocument, 'pointercancel', event => this.handlePointerEnd(event)));
		this._register(toDisposable(() => this.domNode.classList.remove('dragging')));
		this.scrollAutohideScheduler = this._register(new RunOnceScheduler(() => {
			this.domNode.classList.remove('scrolling');
		}, 500));
	}

	public invalidateTokens(): void {
		this.semanticTokensRevision += 1;
	}

	public render(context: EditorRenderingContext): void {
		const minimapLayout = this.options.readMinimapLayout();
		const visible = minimapLayout.renderMinimap !== RenderMinimap.None && minimapLayout.minimapWidth > 0;
		this.domNode.hidden = !visible;
		if (!visible) return;

		const layout = context.layout;
		const renderLayout = this.createRenderLayout(layout, minimapLayout);
		this.updateScrollAutohide(layout.scrollPosition.top);
		this.root.setTransform(`translate3d(${layout.scrollPosition.left + minimapLayout.minimapLeft}px, ${layout.scrollPosition.top}px, 0)`);
		this.root.setWidth(minimapLayout.minimapWidth);
		this.root.setHeight(layout.viewportSize.height);
		this.canvas.style.width = `${minimapLayout.minimapCanvasOuterWidth}px`;
		this.canvas.style.height = `${minimapLayout.minimapCanvasOuterHeight}px`;
		if (this.canvas.width !== minimapLayout.minimapCanvasInnerWidth) this.canvas.width = minimapLayout.minimapCanvasInnerWidth;
		if (this.canvas.height !== minimapLayout.minimapCanvasInnerHeight) this.canvas.height = minimapLayout.minimapCanvasInnerHeight;

		this.renderSlider(renderLayout);
		const layoutKey = minimapLayoutKey(minimapLayout);
		const canvasKey = `${layout.modelVersion}:${this.options.readProjectionRevision()}:${this.semanticTokensRevision}:${layoutKey}:${renderLayout.key}`;
		if (this.renderedCanvasKey !== canvasKey) {
			this.renderCanvas(minimapLayout, renderLayout);
			this.renderedCanvasKey = canvasKey;
		}
		const markerKey = `${this.options.readMarkersRevision()}:${this.options.readProjectionRevision()}:${layoutKey}:${renderLayout.key}`;
		if (this.renderedMarkersKey !== markerKey) {
			this.renderMarkers(renderLayout);
			this.renderedMarkersKey = markerKey;
		}
	}

	private renderCanvas(layout: EditorMinimapLayoutInfo, renderLayout: MinimapRenderLayout): void {
		const ownerWindow = this.canvas.ownerDocument.defaultView;
		if (!ownerWindow || !('CanvasRenderingContext2D' in ownerWindow)) return;
		const context = this.canvas.getContext('2d');
		if (!context) return;
		const width = layout.minimapCanvasInnerWidth;
		const height = layout.minimapCanvasInnerHeight;
		if (width === 0 || height === 0) return;
		const imageData = context.createImageData(width, height);
		const style = ownerWindow.getComputedStyle(this.domNode);
		const foreground = parseCssColor(style.color, new RGBA8(128, 128, 128, 255));
		const background = parseCssColor(style.backgroundColor, new RGBA8(0, 0, 0, 0));
		const fontFamily = style.fontFamily || 'monospace';
		const charRenderer = MinimapCharRendererFactory.create(layout.minimapScale, fontFamily);
		const projection = this.options.readVisualProjection();
		const visualLineCount = projection.visualLineCount;
		const rowCount = layout.minimapIsSampling
			? Math.min(visualLineCount, Math.max(0, Math.floor((height - renderLayout.topPaddingInnerHeight - renderLayout.bottomPaddingInnerHeight) / layout.minimapLineHeight)))
			: renderLayout.endVisualLineIndexExclusive - renderLayout.startVisualLineIndex;
		for (let rowIndex = 0; rowIndex < rowCount; rowIndex += 1) {
			const visualLineIndex = layout.minimapIsSampling
				? Math.min(visualLineCount - 1, Math.floor(rowIndex * visualLineCount / rowCount))
				: renderLayout.startVisualLineIndex + rowIndex;
			const visualLine = projection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const lineText = this.options.model.getLineContent((visualLine.logicalLineIndex) + 1);
			const tokens = this.options.semanticTokenSource?.getLineTokens(visualLine.logicalLineIndex) ?? [];
			this.renderLine(
				imageData,
				renderLayout.topPaddingInnerHeight + rowIndex * layout.minimapLineHeight,
				lineText.slice(visualLine.startColumn, visualLine.endColumn),
				visualLine.startColumn,
				tokens,
				style,
				foreground,
				background,
				charRenderer,
				layout,
			);
		}
		context.putImageData(imageData, 0, 0);
	}

	private updateScrollAutohide(scrollTop: number): void {
		if (this.options.options.autohide !== 'scroll' || scrollTop === this.lastScrollTop) return;
		this.lastScrollTop = scrollTop;
		this.domNode.classList.add('scrolling');
		this.scrollAutohideScheduler.schedule();
	}

	private renderLine(
		target: ImageData,
		dy: number,
		text: string,
		startColumn: number,
		tokens: readonly ResolvedSemanticToken[],
		style: CSSStyleDeclaration,
		foreground: RGBA8,
		background: RGBA8,
		charRenderer: ReturnType<typeof MinimapCharRendererFactory.create>,
		layout: EditorMinimapLayoutInfo,
	): void {
		let visibleColumn = 0;
		for (let offset = 0; offset < text.length && visibleColumn < this.options.options.maxColumn; offset += 1) {
			const code = text.charCodeAt(offset);
			if (code === 9) {
				visibleColumn += this.options.tabSize - visibleColumn % this.options.tabSize;
				continue;
			}
			const dx = visibleColumn * layout.minimapScale;
			if (dx + layout.minimapScale > target.width) break;
			if (code !== 32) {
				const color = tokenColor(tokens, startColumn + offset, style, foreground);
				if (layout.renderMinimap === RenderMinimap.Text) {
					charRenderer.renderChar(target, dx, dy, code, color, 255, background, 0, layout.minimapScale, false, layout.minimapLineHeight === 1);
				} else {
					charRenderer.blockRenderChar(target, dx, dy, color, 255, background, 0, layout.minimapLineHeight === 1);
				}
			}
			visibleColumn += 1;
		}
	}

	private renderMarkers(renderLayout: MinimapRenderLayout): void {
		const markers = fragment(this.domNode.ownerDocument);
		const projection = this.options.readVisualProjection();
		for (const marker of this.options.readMarkers()) {
			const startVisualLineIndex = projection.firstVisualLineIndex(marker.startLineIndex);
			const endVisualLineIndexExclusive = marker.endLineIndexExclusive >= projection.logicalLineCount
				? projection.visualLineCount
				: projection.firstVisualLineIndex(marker.endLineIndexExclusive);
			const span = renderLayout.lineSpan(startVisualLineIndex, Math.max(startVisualLineIndex + 1, endVisualLineIndexExclusive));
			if (!span) continue;
			const element = h(this.domNode.ownerDocument, 'span');
			element.className = `stanza-editor-minimap-diagnostic-marker ${marker.presentation}`;
			element.style.top = `${span.top}px`;
			element.style.height = `${Math.max(2, span.height)}px`;
			if (marker.hoverText !== undefined) element.title = marker.hoverText;
			markers.append(element);
		}
		reset(this.markerLayer, markers);
	}

	private renderSlider(layout: MinimapRenderLayout): void {
		this.slider.hidden = false;
		this.slider.style.height = `${layout.sliderHeight}px`;
		this.slider.style.transform = `translate3d(0, ${layout.sliderTop}px, 0)`;
	}

	private handlePointerDown(event: PointerEvent): void {
		if (event.button !== 0) return;
		const layout = this.options.readLayout();
		if (layout.viewportSize.height <= 0) return;
		const minimapLayout = this.options.readMinimapLayout();
		const renderLayout = this.createRenderLayout(layout, minimapLayout);
		const canvasOffset = this.canvasOffsetAt(event.clientY, layout, minimapLayout);
		const hitsSlider = renderLayout.sliderNeeded
			&& canvasOffset >= renderLayout.sliderTop
			&& canvasOffset <= renderLayout.sliderTop + renderLayout.sliderHeight;
		this.pointerSliderOffset = undefined;
		if (renderLayout.sliderNeeded) {
			this.pointerSliderOffset = hitsSlider ? canvasOffset - renderLayout.sliderTop : renderLayout.sliderHeight / 2;
		}
		this.pointerId = readPointerId(event);
		this.domNode.classList.add('dragging');
		event.preventDefault();
		if (!hitsSlider) {
			this.options.scrollTo({ left: layout.scrollPosition.left, top: renderLayout.scrollTopAt(canvasOffset) });
		}
	}

	private handlePointerMove(event: PointerEvent): void {
		if (this.pointerId === undefined || readPointerId(event) !== this.pointerId) return;
		event.preventDefault();
		const layout = this.options.readLayout();
		const minimapLayout = this.options.readMinimapLayout();
		const renderLayout = this.createRenderLayout(layout, minimapLayout);
		const canvasOffset = this.canvasOffsetAt(event.clientY, layout, minimapLayout);
		const top = this.pointerSliderOffset === undefined
			? renderLayout.scrollTopAt(canvasOffset)
			: renderLayout.scrollTopAtSliderPosition(canvasOffset, this.pointerSliderOffset);
		this.options.scrollTo({ left: layout.scrollPosition.left, top });
	}

	private handlePointerEnd(event: PointerEvent): void {
		if (this.pointerId === undefined || readPointerId(event) !== this.pointerId) return;
		this.pointerId = undefined;
		this.pointerSliderOffset = undefined;
		this.domNode.classList.remove('dragging');
	}

	private canvasOffsetAt(clientY: number, layout: EditorViewportLayout, minimapLayout: EditorMinimapLayoutInfo): number {
		if (!Number.isFinite(clientY) || layout.viewportSize.height <= 0) return 0;
		const bounds = this.domNode.getBoundingClientRect();
		const renderedHeight = bounds.height > 0 ? bounds.height : layout.viewportSize.height;
		return clamp((clientY - bounds.top) / renderedHeight, 0, 1) * minimapLayout.minimapCanvasOuterHeight;
	}

	private createRenderLayout(editorLayout: EditorViewportLayout, minimapLayout: EditorMinimapLayoutInfo): MinimapRenderLayout {
		return MinimapRenderLayout.create({
			editorLayout,
			minimapLayout,
			visualLineCount: this.options.readVisualProjection().visualLineCount,
			paddingTop: this.options.paddingTop,
			paddingBottom: this.options.paddingBottom,
		});
	}
}

function minimapLayoutKey(layout: EditorMinimapLayoutInfo): string {
	return `${layout.renderMinimap}:${layout.minimapLeft}:${layout.minimapWidth}:${layout.minimapScale}:${layout.minimapLineHeight}:${layout.minimapCanvasInnerWidth}:${layout.minimapCanvasInnerHeight}:${layout.minimapIsSampling}`;
}

function tokenColor(tokens: readonly ResolvedSemanticToken[], column: number, style: CSSStyleDeclaration, defaultColor: RGBA8): RGBA8 {
	const token = tokens.find(candidate => candidate.startColumn <= column && candidate.endColumn > column);
	const explicitColor = token?.syntaxPresentation?.foreground;
	if (explicitColor) return parseCssColor(explicitColor, defaultColor);
	if (!token?.presentation) return defaultColor;
	return parseCssColor(style.getPropertyValue(`--zeta-editor-${token.presentation}-foreground`), defaultColor);
}

function parseCssColor(value: string, defaultColor: RGBA8): RGBA8 {
	const normalized = value.trim();
	const shortHex = /^#([0-9a-f]{3})$/i.exec(normalized)?.[1];
	if (shortHex) return new RGBA8(Number.parseInt(shortHex[0]! + shortHex[0]!, 16), Number.parseInt(shortHex[1]! + shortHex[1]!, 16), Number.parseInt(shortHex[2]! + shortHex[2]!, 16), 255);
	const hex = /^#([0-9a-f]{6})$/i.exec(normalized)?.[1];
	if (hex) return new RGBA8(Number.parseInt(hex.slice(0, 2), 16), Number.parseInt(hex.slice(2, 4), 16), Number.parseInt(hex.slice(4, 6), 16), 255);
	const rgb = /^rgba?\(\s*(\d+(?:\.\d+)?)\s*[, ]\s*(\d+(?:\.\d+)?)\s*[, ]\s*(\d+(?:\.\d+)?)(?:\s*[,/]\s*(\d+(?:\.\d+)?%?))?\s*\)$/i.exec(normalized);
	if (!rgb) return defaultColor;
	const alpha = rgb[4]?.endsWith('%') ? Number.parseFloat(rgb[4]) * 2.55 : Number.parseFloat(rgb[4] ?? '1') * 255;
	return new RGBA8(Number(rgb[1]), Number(rgb[2]), Number(rgb[3]), alpha);
}

function readPointerId(event: PointerEvent): number {
	return Number.isSafeInteger(event.pointerId) ? event.pointerId : 0;
}

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
