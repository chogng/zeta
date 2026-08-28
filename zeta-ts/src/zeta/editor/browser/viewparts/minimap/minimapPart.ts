import "./minimap.css";
import { addDisposableListener, h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorScrollPosition } from "../../../common/viewModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/viewLayout.js";
import { type DiagnosticOverviewMarker } from "../overviewRuler/diagnosticOverviewMarkers.js";
import { type DiffOverviewMarker } from "../overviewRuler/diffOverviewMarkers.js";
import { GpuMinimapRenderer } from "./gpuMinimapRenderer.js";
import { MinimapNavigationController } from "./minimapNavigationController.js";
import { MINIMAP_MINIMUM_EDITOR_WIDTH, MINIMAP_WIDTH, createMinimapVerticalLayout, minimapContentWidth } from "./minimapPresentation.js";
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
	readonly verticalScrollbarWidth: number;
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
	private readonly enabled: boolean;
	private readonly verticalScrollbarWidth: number;
	private renderedMarkersRevision = -1;
	private renderedModelVersion = -1;
	private renderedLineScale = -1;

	constructor(options: MinimapPartOptions) {
		super();
		this.model = options.model;
		this.readLayout = options.readLayout;
		this.readRenderingContext = options.readRenderingContext;
		this.readMarkers = options.readMarkers;
		this.readMarkersRevision = options.readMarkersRevision;
		this.enabled = options.enabled;
		this.verticalScrollbarWidth = options.verticalScrollbarWidth;
		const ownerDocument = options.host.ownerDocument;
		const domNode = h(ownerDocument, "div");
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.canvas = h(ownerDocument, "canvas");
		this.viewportElement = h(ownerDocument, "div");
		this.viewportNode = new FastDomNode(this.viewportElement);
		this.root.setClassName("stanza-editor-minimap");
		this.domNode.hidden = !options.enabled;
		this.domNode.setAttribute("aria-hidden", "true");
		this.canvas.className = "stanza-editor-minimap-gpu";
		this.canvas.setAttribute("aria-hidden", "true");
		this.viewportNode.setClassName("stanza-editor-minimap-viewport");
		this.domNode.append(this.canvas, this.viewportElement);
		const gpuRenderer = options.enabled
			? GpuMinimapRenderer.tryCreate(this.canvas)
			: undefined;
		this.gpuRenderer = gpuRenderer;
		if (gpuRenderer) this._register(toDisposable(() => gpuRenderer.dispose()));
		this._register(new MinimapNavigationController(
			this.domNode,
			options.readLayout,
			options.scrollTo,
		));
		this._register(addDisposableListener<globalThis.Event>(this.canvas, "webglcontextlost", event => {
			event.preventDefault();
			this.gpuRenderer?.disable();
			this.renderedMarkersRevision = -1;
			this.renderNow(this.readRenderingContext());
		}));
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		const visible = this.enabled && layout.viewportSize.width >= MINIMAP_MINIMUM_EDITOR_WIDTH;
		const hidden = !visible;
		if (this.domNode.hidden !== hidden) this.domNode.hidden = hidden;
		if (!visible) return;
		const left = layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - this.verticalScrollbarWidth - MINIMAP_WIDTH);
		this.root.setTransform(`translate3d(${left}px, ${layout.scrollPosition.top}px, 0)`);
		this.root.setHeight(layout.viewportSize.height);
		const minimapLayout = createMinimapVerticalLayout(
			layout.viewportSize.height,
			layout.contentSize.height,
			layout.scrollPosition.top,
			layout.lineHeight,
			this.model.lineCount,
		);
		const slider = minimapLayout.slider;
		this.viewportElement.hidden = !slider.visible;
		this.viewportNode.setHeight(slider.height);
		this.viewportNode.setTransform(`translate3d(0, ${slider.top}px, 0)`);
		this.gpuRenderer?.resize(MINIMAP_WIDTH, layout.viewportSize.height);
		const markersRevision = this.readMarkersRevision();
		if (
			this.renderedMarkersRevision === markersRevision &&
			this.renderedModelVersion === layout.modelVersion &&
			this.renderedLineScale === minimapLayout.lineScale
		) return;
		const fragment = createFragment(this.domNode.ownerDocument);
		const rows = createMinimapRows(this.model);
		if (this.gpuRenderer?.isAvailable) {
			this.gpuRenderer.setRows(rows, minimapLayout.lineScale);
		} else {
			for (const row of rows) {
				const marker = h(this.domNode.ownerDocument, "span");
				marker.className = "stanza-editor-minimap-row";
				marker.style.top = `${row.startLineIndex * minimapLayout.lineScale}px`;
				marker.style.width = `${minimapContentWidth(row.density)}px`;
				marker.style.height = `${Math.max(1, (row.endLineIndexExclusive - row.startLineIndex) * minimapLayout.lineScale)}px`;
				fragment.append(marker);
			}
		}
		for (const marker of this.readMarkers()) {
			const element = h(this.domNode.ownerDocument, "span");
			element.className = "stanza-editor-minimap-diagnostic-marker";
			element.classList.add(marker.presentation);
			element.style.top = `${marker.startLineIndex * minimapLayout.lineScale}px`;
			element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) * minimapLayout.lineScale)}px`;
			if (marker.hoverText !== undefined) element.title = marker.hoverText;
			fragment.append(element);
		}
		reset(this.domNode, this.canvas, fragment, this.viewportElement);
		this.renderedMarkersRevision = markersRevision;
		this.renderedModelVersion = layout.modelVersion;
		this.renderedLineScale = minimapLayout.lineScale;
	}
}
