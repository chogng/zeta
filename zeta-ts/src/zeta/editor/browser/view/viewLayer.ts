import '../viewparts/viewLines/viewLines.css';
import { h, reset, fragment as createFragment } from '../../../base/browser/dom.js';
import { FastDomNode } from '../../../base/browser/fastDomNode.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { type EditorVisualLine, type EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type EditorLineRange } from '../../common/viewModel.js';
import { type ViewportData } from '../../common/viewLayout/viewLinesViewportData.js';
import { type EditorRenderingContext } from './renderingContext.js';

export interface ViewLayerLineRenderer<TLine> {
	createLine(visualLineIndex: number): TLine;
	getDomNode(line: TLine): HTMLElement;
	renderLine(line: TLine, visualLine: EditorVisualLine): void;
	layoutLine(line: TLine, lineHeight: number): void;
}

export interface ViewLayerOptions<TLine> {
	readonly host: HTMLElement;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readProjectionRevision: () => number;
	readonly lineRenderer: ViewLayerLineRenderer<TLine>;
}

/**
 * Reconciles the virtualized visual-line window and owns only its layer DOM.
 *
 * Content-specific rendering belongs to the ViewLines part; overlays consume
 * the stable line objects exposed by this layer.
 */
export class ViewLayer<TLine> extends Disposable {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly readProjectionRevision: () => number;
	private readonly lineRenderer: ViewLayerLineRenderer<TLine>;
	private lines = new Map<number, TLine>();
	private renderedRange: EditorLineRange = { startLineIndex: 0, endLineIndexExclusive: 0 };
	private renderedModelVersion = -1;
	private renderedLineHeight = -1;
	private renderedProjectionRevision = -1;
	private renderedVerticalOffsets: readonly number[] | undefined;

	constructor(options: ViewLayerOptions<TLine>) {
		super();
		this.readVisualProjection = options.readVisualProjection;
		this.readProjectionRevision = options.readProjectionRevision;
		this.lineRenderer = options.lineRenderer;
		const domNode = h(options.host.ownerDocument, 'div');
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName('stanza-editor-lines');
	}

	get renderedLines(): ReadonlyMap<number, TLine> {
		return this.lines;
	}

	get renderedLineRange(): EditorLineRange {
		return this.renderedRange;
	}

	render(viewportData: ViewportData): void {
		this.root.setTop(viewportData.renderTop);
		const visualProjection = this.readVisualProjection();
		const projectionRevision = this.readProjectionRevision();
		if (visualProjection.modelVersion !== viewportData.modelVersion) return;
		if (
			this.renderedModelVersion === viewportData.modelVersion &&
			this.renderedLineHeight === viewportData.lineHeight &&
			this.renderedProjectionRevision === projectionRevision &&
			numberArraysEqual(this.renderedVerticalOffsets, viewportData.relativeVerticalOffset) &&
			lineRangesEqual(this.renderedRange, viewportData.renderLines)
		) return;

		const fragment = createFragment(this.domNode.ownerDocument);
		const next = new Map<number, TLine>();
		for (let visualLineIndex = viewportData.renderLines.startLineIndex; visualLineIndex < viewportData.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) throw new Error('Viewport render range exceeds the visual line projection');
			const existing = this.lines.get(visualLineIndex);
			const line = existing ?? this.lineRenderer.createLine(visualLineIndex);
			const needsLineRender = !existing || this.renderedModelVersion !== viewportData.modelVersion || this.renderedProjectionRevision !== projectionRevision;
			if (needsLineRender) this.lineRenderer.renderLine(line, visualLine);
			if (!existing || this.renderedLineHeight !== viewportData.lineHeight) this.lineRenderer.layoutLine(line, viewportData.lineHeight);
			const domNode = this.lineRenderer.getDomNode(line);
			domNode.style.top = `${viewportData.getLineTop(visualLineIndex) - viewportData.renderTop}px`;
			next.set(visualLineIndex, line);
			fragment.append(domNode);
		}
		reset(this.domNode, fragment);
		this.lines = next;
		this.renderedRange = viewportData.renderLines;
		this.renderedModelVersion = viewportData.modelVersion;
		this.renderedLineHeight = viewportData.lineHeight;
		this.renderedProjectionRevision = projectionRevision;
		this.renderedVerticalOffsets = viewportData.relativeVerticalOffset;
	}
}

function numberArraysEqual(left: readonly number[] | undefined, right: readonly number[]): boolean {
	return left !== undefined && left.length === right.length && left.every((value, index) => value === right[index]);
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
	return left.startLineIndex === right.startLineIndex && left.endLineIndexExclusive === right.endLineIndexExclusive;
}
export class ViewPartRows extends Disposable {
	public readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private rows = new Map<number, FastDomNode<HTMLDivElement>>();

	constructor(host: HTMLElement, className: string, private readonly rowClassName: string) {
		super();
		const domNode = h(host.ownerDocument, 'div');
		this.domNode = domNode;
		this.root = new FastDomNode(domNode);
		this.root.setClassName(`stanza-editor-row-layer ${className}`);
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public render(context: EditorRenderingContext): ReadonlyMap<number, HTMLElement> {
		const fragment = createFragment(this.domNode.ownerDocument);
		const next = new Map<number, FastDomNode<HTMLDivElement>>();
		const projected = new Map<number, HTMLElement>();
		this.root.setTop(context.layout.renderTop);
		for (let lineIndex = context.layout.renderLines.startLineIndex; lineIndex < context.layout.renderLines.endLineIndexExclusive; lineIndex += 1) {
			let row = this.rows.get(lineIndex);
			if (!row) {
				const element = h(this.domNode.ownerDocument, 'div');
				element.className = this.rowClassName;
				element.dataset.lineIndex = String(lineIndex);
				row = new FastDomNode(element);
			}
			row.setHeight(context.layout.lineHeight);
			row.setLineHeight(context.layout.lineHeight);
			row.setPosition('absolute');
			row.setTop(context.viewportData.getLineTop(lineIndex) - context.layout.renderTop);
			next.set(lineIndex, row);
			projected.set(lineIndex, row.domNode);
			fragment.append(row.domNode);
		}
		reset(this.domNode, fragment);
		this.rows = next;
		return projected;
	}
}
