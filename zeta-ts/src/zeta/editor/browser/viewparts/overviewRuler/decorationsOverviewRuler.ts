import "./overviewRuler.css";
import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { type DiagnosticOverviewMarker } from "./diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "./diffOverviewMarkers.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

const OVERVIEW_RULER_WIDTH = 6;

export type OverviewRulerMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface DecorationsOverviewRulerOptions {
	readonly host: HTMLElement;
	readonly verticalScrollbarWidth: number;
	readonly readLineCount: () => number;
	readonly readMarkers: () => readonly OverviewRulerMarker[];
	readonly readMarkersRevision: () => number;
}

/** Projects diagnostic and diff markers into the editor's overview ruler. */
export class DecorationsOverviewRuler extends EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly verticalScrollbarWidth: number;
	private readonly readLineCount: () => number;
	private readonly readMarkers: () => readonly OverviewRulerMarker[];
	private readonly readMarkersRevision: () => number;
	private renderedMarkersRevision = -1;

	constructor(options: DecorationsOverviewRulerOptions) {
		super();
		this.verticalScrollbarWidth = options.verticalScrollbarWidth;
		this.readLineCount = options.readLineCount;
		this.readMarkers = options.readMarkers;
		this.readMarkersRevision = options.readMarkersRevision;
		const domNode = h(options.host.ownerDocument, "div");
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName("stanza-editor-overview-ruler");
		this.domNode.setAttribute("aria-hidden", "true");
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		this.root.setLeft(
			layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - this.verticalScrollbarWidth + (this.verticalScrollbarWidth - OVERVIEW_RULER_WIDTH) / 2),
		);
		this.root.setTop(layout.scrollPosition.top);
		this.root.setHeight(layout.viewportSize.height);
		const markersRevision = this.readMarkersRevision();
		if (this.renderedMarkersRevision === markersRevision) return;
		const lineCount = Math.max(1, this.readLineCount());
		const markers = this.readMarkers();
		const fragment = createFragment(this.domNode.ownerDocument);
		for (const marker of markers) {
			const element = h(this.domNode.ownerDocument, "span");
			element.className = "stanza-editor-overview-marker";
			element.classList.add(marker.presentation);
			element.style.top = `${marker.startLineIndex / lineCount * 100}%`;
			element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / lineCount * 100)}%`;
			if (marker.hoverText !== undefined) element.title = marker.hoverText;
			fragment.append(element);
		}
		reset(this.domNode, fragment);
		this.renderedMarkersRevision = markersRevision;
	}
}
