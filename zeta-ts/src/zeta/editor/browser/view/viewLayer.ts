import '../viewparts/viewLines/viewLines.css';
import { h, reset, fragment as createFragment } from '../../../base/browser/dom.js';
import { FastDomNode } from '../../../base/browser/fastDomNode.js';
import { DisposableOwner } from '../../../base/common/lifecycle.js';
import { type EditorVisualLine, type EditorVisualLineProjection } from '../../common/viewModel/modelLineProjection.js';
import { type EditorLineRange, type EditorViewportLayout } from '../../common/viewLayout/editorViewportModel.js';

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
export class ViewLayer<TLine> extends DisposableOwner {
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

	constructor(options: ViewLayerOptions<TLine>) {
		super();
		this.readVisualProjection = options.readVisualProjection;
		this.readProjectionRevision = options.readProjectionRevision;
		this.lineRenderer = options.lineRenderer;
		this.domNode = this.adopt(h(options.host.ownerDocument, 'div'), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName('stanza-editor-lines');
	}

	get renderedLines(): ReadonlyMap<number, TLine> {
		return this.lines;
	}

	get renderedLineRange(): EditorLineRange {
		return this.renderedRange;
	}

	render(layout: EditorViewportLayout): void {
		this.root.setTransform(`translate3d(0, ${layout.renderTop}px, 0)`);
		const visualProjection = this.readVisualProjection();
		const projectionRevision = this.readProjectionRevision();
		if (visualProjection.modelVersion !== layout.modelVersion) return;
		if (
			this.renderedModelVersion === layout.modelVersion &&
			this.renderedLineHeight === layout.lineHeight &&
			this.renderedProjectionRevision === projectionRevision &&
			lineRangesEqual(this.renderedRange, layout.renderLines)
		) return;

		const fragment = createFragment(this.domNode.ownerDocument);
		const next = new Map<number, TLine>();
		for (let visualLineIndex = layout.renderLines.startLineIndex; visualLineIndex < layout.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
			const visualLine = visualProjection.lineAt(visualLineIndex);
			if (!visualLine) throw new Error('Viewport render range exceeds the visual line projection');
			const existing = this.lines.get(visualLineIndex);
			const line = existing ?? this.lineRenderer.createLine(visualLineIndex);
			const needsLineRender = !existing || this.renderedModelVersion !== layout.modelVersion || this.renderedProjectionRevision !== projectionRevision;
			if (needsLineRender) this.lineRenderer.renderLine(line, visualLine);
			if (!existing || this.renderedLineHeight !== layout.lineHeight) this.lineRenderer.layoutLine(line, layout.lineHeight);
			next.set(visualLineIndex, line);
			fragment.append(this.lineRenderer.getDomNode(line));
		}
		reset(this.domNode, fragment);
		this.lines = next;
		this.renderedRange = layout.renderLines;
		this.renderedModelVersion = layout.modelVersion;
		this.renderedLineHeight = layout.lineHeight;
		this.renderedProjectionRevision = projectionRevision;
	}
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
	return left.startLineIndex === right.startLineIndex && left.endLineIndexExclusive === right.endLineIndexExclusive;
}

