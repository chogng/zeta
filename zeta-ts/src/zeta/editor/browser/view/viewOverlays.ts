import { createFastDomNode, type FastDomNode } from '../../../base/browser/fastDomNode.js';
import { type ViewContext } from '../../common/viewModel/viewContext.js';
import { DynamicViewOverlay } from './dynamicViewOverlay.js';
import { type EditorRenderingContext, ViewPart } from './viewPart.js';
import { ViewPartRows } from './viewLayer.js';

export class ViewOverlays extends ViewPart {
	protected readonly domNode: FastDomNode<HTMLElement>;
	private readonly rows: ViewPartRows;
	private dynamicOverlays: DynamicViewOverlay[] = [];

	constructor(context: ViewContext, host: HTMLElement, className = 'view-overlays') {
		super(context);
		this.rows = this._register(new ViewPartRows(host, className, 'view-overlay-line'));
		this.domNode = createFastDomNode(this.rows.domNode);
	}

	public override shouldRender(): boolean {
		return super.shouldRender() || this.dynamicOverlays.some(overlay => overlay.shouldRender());
	}

	public override dispose(): void {
		for (const overlay of this.dynamicOverlays.splice(0)) overlay.dispose();
		super.dispose();
	}

	public getDomNode(): FastDomNode<HTMLElement> {
		return this.domNode;
	}

	public addDynamicOverlay(overlay: DynamicViewOverlay): void {
		this.dynamicOverlays.push(overlay);
	}

	public prepareRender(context: EditorRenderingContext): void {
		for (const overlay of this.dynamicOverlays) {
			overlay.prepareRender(context);
			overlay.onDidRender();
		}
	}

	public render(context: EditorRenderingContext): void {
		const startLineNumber = context.layout.renderLines.startLineIndex + 1;
		for (const [lineIndex, row] of this.rows.render(context)) {
			const lineNumber = lineIndex + 1;
			row.innerHTML = this.dynamicOverlays
				.map(overlay => overlay.render(startLineNumber, lineNumber))
				.join('');
		}
	}
}

export class ContentViewOverlays extends ViewOverlays {
	constructor(context: ViewContext, host: HTMLElement) {
		super(context, host, 'view-overlays');
		this.domNode.setHeight(0);
	}

	public override render(context: EditorRenderingContext): void {
		super.render(context);
		this.domNode.setWidth(Math.max(context.layout.contentSize.width, context.layout.viewportSize.width));
	}
}

export class MarginViewOverlays extends ViewOverlays {
	constructor(context: ViewContext, host: HTMLElement) {
		super(context, host, 'margin-view-overlays');
	}

	public override render(context: EditorRenderingContext): void {
		super.render(context);
		this.domNode.setHeight(Math.min(context.layout.contentSize.height, 1_000_000));
	}
}
