import { type FastDomNode } from '../../../base/browser/fastDomNode.js';
import { ViewEventHandler } from '../../common/viewEventHandler.js';
import { type ViewContext } from '../../common/viewModel/viewContext.js';
import { type EditorRenderingContext } from './renderingContext.js';

export type { EditorRenderingContext } from './renderingContext.js';

export abstract class ViewPart extends ViewEventHandler {
	protected readonly _context: ViewContext;

	constructor(context: ViewContext) {
		super();
		this._context = context;
		this._context.addEventHandler(this);
	}

	public override dispose(): void {
		this._context.removeEventHandler(this);
		super.dispose();
	}

	public onBeforeRender(_viewportData: EditorRenderingContext['viewportData']): void {
	}

	public prepareRender(_context: EditorRenderingContext): void {
	}

	public renderNow(context: EditorRenderingContext): void {
		this.prepareRender(context);
		this.render(context);
	}

	public abstract render(context: EditorRenderingContext): void;
}

export const enum PartFingerprint {
	None,
	ContentWidgets,
	OverflowingContentWidgets,
	OverflowGuard,
	OverlayWidgets,
	OverflowingOverlayWidgets,
	ScrollableElement,
	TextArea,
	ViewLines,
	Minimap,
	ViewLinesGpu,
}

export class PartFingerprints {
	public static write(target: Element | FastDomNode<HTMLElement>, fingerprint: PartFingerprint) {
		target.setAttribute('data-mprt', String(fingerprint));
	}

	public static read(target: Element): PartFingerprint {
		const value = target.getAttribute('data-mprt');
		return value === null ? PartFingerprint.None : Number.parseInt(value, 10);
	}

	public static collect(child: Element | null, stopAt: Element): Uint8Array {
		const fingerprints: PartFingerprint[] = [];
		while (child && child !== child.ownerDocument.body) {
			if (child === stopAt) break;
			fingerprints.push(this.read(child));
			child = child.parentElement;
		}
		return Uint8Array.from(fingerprints.reverse());
	}
}
