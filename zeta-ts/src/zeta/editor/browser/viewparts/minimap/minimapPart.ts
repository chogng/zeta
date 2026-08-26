import "./minimap.css";
import { addDisposableListener, h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorScrollPosition, type EditorViewportLayout } from "../../../common/viewLayout/viewLayout.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { GpuMinimapRenderer } from "./gpuMinimapRenderer.js";
import { MinimapNavigationController } from "./minimapNavigationController.js";
import { MINIMAP_LINE_HEIGHT, MINIMAP_WIDTH, createMinimapSliderLayout, minimapContentWidth } from "./minimapPresentation.js";
import { createMinimapRows } from "./minimapProjection.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

export type MinimapMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface MinimapPartOptions {
	readonly host: HTMLElement;
	readonly model: TextModel;
	readonly readLayout: () => EditorViewportLayout;
	readonly readRenderingContext: () => EditorRenderingContext;
	readonly scrollTo: (position: EditorScrollPosition) => void;
	readonly readMarkers: () => readonly MinimapMarker[];
	readonly readMarkersRevision: () => number;
	readonly enabled: boolean;
}

/** Owns the minimap preview, its bounded density projection, and navigation. */
export class MinimapPart extends EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly canvas: HTMLCanvasElement;
	private readonly viewportElement: HTMLDivElement;
	private readonly viewportNode: FastDomNode<HTMLDivElement>;
	private readonly model: TextModel;
	private readonly readLayout: () => EditorViewportLayout;
	private readonly readRenderingContext: () => EditorRenderingContext;
	private readonly readMarkers: () => readonly MinimapMarker[];
	private readonly readMarkersRevision: () => number;
	private readonly gpuRenderer: GpuMinimapRenderer | undefined;
	private renderedMarkersRevision = -1;

	constructor(options: MinimapPartOptions) {
		super();
		this.model = options.model;
		this.readLayout = options.readLayout;
		this.readRenderingContext = options.readRenderingContext;
		this.readMarkers = options.readMarkers;
		this.readMarkersRevision = options.readMarkersRevision;
		const ownerDocument = options.host.ownerDocument;
		this.domNode = this.adopt(h(ownerDocument, "div"), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.canvas = h(ownerDocument, "canvas");
		this.viewportElement = h(ownerDocument, "div");
		this.viewportNode = new FastDomNode(this.viewportElement);
		this.root.setClassName("stanza-editor-minimap");
		this.root.setHidden(!options.enabled);
		this.domNode.setAttribute("aria-hidden", "true");
		this.canvas.className = "stanza-editor-minimap-gpu";
		this.canvas.setAttribute("aria-hidden", "true");
		this.viewportNode.setClassName("stanza-editor-minimap-viewport");
		this.domNode.append(this.canvas, this.viewportElement);
		this.gpuRenderer = options.enabled
			? GpuMinimapRenderer.tryCreate(this.canvas)
			: undefined;
		this.own(new MinimapNavigationController(
			this.domNode,
			options.readLayout,
			options.scrollTo,
		));
		this.own(addDisposableListener<globalThis.Event>(this.canvas, "webglcontextlost", event => {
			event.preventDefault();
			this.gpuRenderer?.disable();
			this.renderedMarkersRevision = -1;
			this.renderNow(this.readRenderingContext());
		}));
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		if (this.domNode.hidden) return;
		const left = layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - MINIMAP_WIDTH);
		this.root.setTransform(`translate3d(${left}px, ${layout.scrollPosition.top}px, 0)`);
		this.root.setHeight(layout.viewportSize.height);
		const slider = createMinimapSliderLayout(
			layout.viewportSize.height,
			layout.contentSize.height,
			layout.scrollPosition.top,
		);
		this.viewportNode.setHeight(slider.height);
		this.viewportNode.setTransform(`translate3d(0, ${slider.top}px, 0)`);
		this.gpuRenderer?.resize(MINIMAP_WIDTH, layout.viewportSize.height);
		const markersRevision = this.readMarkersRevision();
		if (this.renderedMarkersRevision === markersRevision) return;
		const fragment = createFragment(this.domNode.ownerDocument);
		const rows = createMinimapRows(this.model);
		if (this.gpuRenderer?.isAvailable) {
			this.gpuRenderer.setRows(rows, this.model.lineCount);
		} else {
			for (const row of rows) {
				const marker = h(this.domNode.ownerDocument, "span");
				marker.className = "stanza-editor-minimap-row";
				marker.style.top = `${row.startLineIndex / this.model.lineCount * 100}%`;
				marker.style.width = `${minimapContentWidth(row.density)}px`;
				marker.style.height = `${MINIMAP_LINE_HEIGHT}px`;
				fragment.append(marker);
			}
		}
		for (const marker of this.readMarkers()) {
			const element = h(this.domNode.ownerDocument, "span");
			element.className = "stanza-editor-minimap-diagnostic-marker";
			element.classList.add(marker.presentation);
			element.style.top = `${marker.startLineIndex / this.model.lineCount * 100}%`;
			element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / this.model.lineCount * 100)}%`;
			if (marker.hoverText !== undefined) element.title = marker.hoverText;
			fragment.append(element);
		}
		reset(this.domNode, this.canvas, fragment, this.viewportElement);
		this.renderedMarkersRevision = markersRevision;
	}
}
