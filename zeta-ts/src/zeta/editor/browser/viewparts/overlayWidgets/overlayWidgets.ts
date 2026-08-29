import './overlayWidgets.css';
import { h } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type EditorRenderingContext, EditorViewPart } from '../../view/viewPart.js';

export type OverlayWidgetPositionPreference = 'top-right' | 'bottom-right' | 'top-left' | 'bottom-left';

export interface IOverlayWidget {
	readonly id: string;
	readonly domNode: HTMLElement;
	getPosition(): OverlayWidgetPositionPreference | { readonly left: number; readonly top: number } | undefined;
}

/** Owns widgets positioned against the editor viewport rather than document content. */
export class ViewOverlayWidgets extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly widgets = new Map<string, IOverlayWidget>();

	constructor(ownerDocument: Document) {
		super();
		this.domNode = h(ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-overlay-widgets';
		this.domNode.setAttribute('role', 'presentation');
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public addWidget(widget: IOverlayWidget): void {
		if (!widget.id || this.widgets.has(widget.id)) {
			throw new RangeError(`Overlay widget '${widget.id}' is already registered`);
		}
		this.widgets.set(widget.id, widget);
		this.domNode.append(widget.domNode);
	}

	public removeWidget(widget: IOverlayWidget): void {
		if (this.widgets.get(widget.id) !== widget) {
			return;
		}
		this.widgets.delete(widget.id);
		widget.domNode.remove();
	}

	public render(context: EditorRenderingContext): void {
		for (const widget of this.widgets.values()) {
			const position = widget.getPosition();
			widget.domNode.hidden = position === undefined;
			if (position === undefined) {
				continue;
			}
			widget.domNode.style.position = 'absolute';
			const coordinates = typeof position === 'string' ? cornerCoordinates(position, context.layout.viewportSize.width, context.layout.viewportSize.height, widget.domNode) : position;
			widget.domNode.style.left = `${coordinates.left}px`;
			widget.domNode.style.top = `${coordinates.top}px`;
		}
	}
}

function cornerCoordinates(position: OverlayWidgetPositionPreference, width: number, height: number, domNode: HTMLElement): { readonly left: number; readonly top: number } {
	const left = position.endsWith('right') ? Math.max(0, width - domNode.offsetWidth) : 0;
	const top = position.startsWith('bottom') ? Math.max(0, height - domNode.offsetHeight) : 0;
	return { left, top };
}
