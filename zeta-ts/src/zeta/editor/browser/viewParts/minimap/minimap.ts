import './minimap.css';

import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { FastDomNode } from '../../../../base/browser/fastDomNode.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { clamp } from '../../../../base/common/numbers.js';
import { type EditorMinimapLayoutInfo, type EditorMinimapOptions, RenderMinimap } from '../../../common/config/editorOptions.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { type SemanticTokenSource } from '../../../common/services/resolvedSemanticTokens.js';
import { type EditorScrollPosition } from '../../../common/viewModel/editorViewportContracts.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import { type EditorViewportLayout } from '../../../common/viewLayout/viewLayout.js';
import { type RestrictedRenderingContext } from '../../view/renderingContext.js';
import { ViewPart, PartFingerprint, PartFingerprints } from '../../view/viewPart.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { Range } from '../../../common/core/range.js';

export interface MinimapOptions {
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
	readonly scrollTo: (position: EditorScrollPosition) => void;
}

/** Owns the document overview canvas, viewport indicator, markers, and pointer navigation. */
export class Minimap extends ViewPart {
	private readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly canvas: HTMLCanvasElement;
	private dragging = false;

	constructor(context: ViewContext, private readonly source: MinimapOptions) {
		super(context);
		this.domNode = h(source.host.ownerDocument, 'div');
		this.root = new FastDomNode(this.domNode);
		this.domNode.className = 'minimap';
		this.root.setPosition('absolute');
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		PartFingerprints.write(this.domNode, PartFingerprint.Minimap);
		this.canvas = h(source.host.ownerDocument, 'canvas');
		this.canvas.style.position = 'absolute';
		this.canvas.style.left = '0';
		this.domNode.append(this.canvas);
		source.host.append(this.domNode);
		this._register(toDisposable(() => this.domNode.remove()));
		this._register(addDisposableListener(this.domNode, 'pointerdown', event => {
			if (event.button !== 0) return;
			this.dragging = true;
			this.domNode.setPointerCapture(event.pointerId);
			this.moveTo(event.clientY);
			event.preventDefault();
		}));
		this._register(addDisposableListener(this.domNode, 'pointermove', event => {
			if (this.dragging) this.moveTo(event.clientY);
		}));
		this._register(addDisposableListener(this.domNode, 'pointerup', event => {
			this.dragging = false;
			if (this.domNode.hasPointerCapture(event.pointerId)) this.domNode.releasePointerCapture(event.pointerId);
		}));
		this._register(addDisposableListener(this.domNode, 'pointercancel', () => this.dragging = false));
	}

	public getDomNode(): FastDomNode<HTMLElement> {
		return this.root;
	}

	render(context: RestrictedRenderingContext): void {
		const geometry = this.source.readMinimapLayout();
		const visible = this.source.options.enabled && geometry.renderMinimap !== RenderMinimap.None && geometry.minimapWidth > 0;
		this.domNode.style.display = visible ? '' : 'none';
		if (!visible) return;

		this.domNode.style.left = `${context.scrollLeft + geometry.minimapLeft}px`;
		this.domNode.style.top = `${context.scrollTop}px`;
		this.domNode.style.width = `${geometry.minimapWidth}px`;
		this.domNode.style.height = `${context.viewportHeight}px`;
		this.canvas.style.width = `${geometry.minimapCanvasOuterWidth}px`;
		this.canvas.style.height = `${geometry.minimapCanvasOuterHeight}px`;
		this.canvas.width = Math.max(1, Math.round(geometry.minimapCanvasInnerWidth));
		this.canvas.height = Math.max(1, Math.round(geometry.minimapCanvasInnerHeight));
		this.paint(context, geometry);
	}

	private paint(context: RestrictedRenderingContext, geometry: EditorMinimapLayoutInfo): void {
		const painter = this.canvas.getContext('2d');
		if (!painter) return;
		const width = this.canvas.width;
		const height = this.canvas.height;
		painter.clearRect(0, 0, width, height);
		const projection = this.source.readVisualProjection();
		const scaleY = height / Math.max(1, projection.visualLineCount + this.source.paddingTop + this.source.paddingBottom);
		const rowHeight = Math.max(1, geometry.minimapLineHeight * geometry.minimapScale);
		const charWidth = Math.max(1, geometry.minimapScale);
		const styles = this.source.host.ownerDocument.defaultView!.getComputedStyle(this.source.host);
		const foreground = styles.getPropertyValue('--vscode-editor-foreground').trim() || styles.color || '#808080';
		painter.fillStyle = foreground;
		painter.globalAlpha = 0.55;
		for (const line of projection.lines) {
			const text = this.source.model.getLineContent(line.logicalLineIndex + 1).slice(line.startColumn, line.endColumn);
			const indentation = leadingWidth(text, this.source.tabSize);
			const visibleWidth = Math.max(1, Math.min(width - indentation * charWidth, (text.length - indentation) * charWidth));
			const y = Math.floor((line.visualLineIndex + this.source.paddingTop) * scaleY);
			painter.fillRect(indentation * charWidth, y, visibleWidth, rowHeight);
		}
		painter.globalAlpha = 1;

		const fullRange = new Range(1, 1, this._context.viewModel.getLineCount(), this._context.viewModel.getLineMaxColumn(this._context.viewModel.getLineCount()));
		for (const decoration of this._context.viewModel.getMinimapDecorationsInRange(fullRange)) {
			const minimap = decoration.options.minimap;
			if (!minimap?.color) continue;
			const color = typeof minimap.color === 'string' ? minimap.color : this._context.theme.getColor(minimap.color.id)?.toString();
			if (!color) continue;
			painter.fillStyle = color;
			const top = Math.floor((decoration.range.startLineNumber - 1) / Math.max(1, this._context.viewModel.getLineCount()) * height);
			const markerHeight = Math.max(2, Math.ceil((decoration.range.endLineNumber - decoration.range.startLineNumber + 1) * scaleY));
			painter.fillRect(Math.max(0, width - 3), top, 3, markerHeight);
		}

		const contentHeight = Math.max(context.viewportHeight, context.scrollHeight);
		const sliderTop = context.scrollTop / contentHeight * height;
		const sliderHeight = Math.max(8, context.viewportHeight / contentHeight * height);
		painter.fillStyle = styles.getPropertyValue('--vscode-minimapSlider-background').trim() || 'rgba(128, 128, 128, 0.25)';
		painter.fillRect(0, sliderTop, width, Math.min(height - sliderTop, sliderHeight));
	}

	private moveTo(clientY: number): void {
		const bounds = this.domNode.getBoundingClientRect();
		if (bounds.height <= 0) return;
		const layout = this.source.readLayout();
		const ratio = clamp((clientY - bounds.top) / bounds.height, 0, 1);
		const top = ratio * layout.contentSize.height - layout.viewportSize.height / 2;
		this.source.scrollTo({ left: layout.scrollPosition.left, top: clamp(top, 0, layout.maximumScrollPosition.top) });
	}
}

function leadingWidth(text: string, tabSize: number): number {
	let width = 0;
	for (const character of text) {
		if (character === ' ') width += 1;
		else if (character === '\t') width += tabSize - width % tabSize;
		else break;
	}
	return width;
}
