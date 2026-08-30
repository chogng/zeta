import { type FastDomNode } from '../../../base/browser/fastDomNode.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type EditorViewportLayout } from '../../common/viewLayout/viewLayout.js';
import { type EditorRenderingContext } from './renderingContext.js';

export type { EditorRenderingContext } from './renderingContext.js';

export class EditorViewContext {
	constructor(
		private readonly readLayout: () => EditorViewportLayout,
		private readonly createRenderingContext: (layout: EditorViewportLayout) => EditorRenderingContext,
	) { }

	public get layout(): EditorViewportLayout {
		return this.readLayout();
	}

	public get renderingContext(): EditorRenderingContext {
		return this.createRenderingContext(this.layout);
	}
}

export abstract class EditorViewPart extends Disposable {
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
	EditorViewLines,
	Minimap,
	StyledViewLinesGpu,
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

export class EditorViewPartCollection extends Disposable {
	private readonly parts: EditorViewPart[] = [];

	public register<TPart extends EditorViewPart>(part: TPart): TPart {
		this.parts.push(part);
		this._register(part);
		return part;
	}

	public prepareRender(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.prepareRender(context);
		}
	}

	public render(context: EditorRenderingContext): void {
		for (const part of this.parts) {
			part.render(context);
		}
	}
}
