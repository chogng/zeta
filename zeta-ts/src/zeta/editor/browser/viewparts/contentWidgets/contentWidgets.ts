import { h } from '../../../../base/browser/dom.js';
import { toDisposable } from '../../../../base/common/lifecycle.js';
import { type TextPosition } from '../../../common/core/text.js';
import { type EditorRenderingContext, EditorViewPart } from '../../view/viewPart.js';

export type ContentWidgetPositionPreference = 'above' | 'below' | 'exact';

export interface IContentWidget {
	readonly id: string;
	readonly domNode: HTMLElement;
	getPosition(): { readonly position: TextPosition; readonly preference?: ContentWidgetPositionPreference } | undefined;
}

/** Owns editor content-widget DOM and positions it against the current rendered view. */
export class ViewContentWidgets extends EditorViewPart {
	public readonly domNode: HTMLDivElement;
	private readonly widgets = new Map<string, IContentWidget>();

	constructor(ownerDocument: Document) {
		super();
		this.domNode = h(ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-content-widgets';
		this.domNode.setAttribute('role', 'presentation');
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public addWidget(widget: IContentWidget): void {
		if (!widget.id || this.widgets.has(widget.id)) {
			throw new RangeError(`Content widget '${widget.id}' is already registered`);
		}
		this.widgets.set(widget.id, widget);
		this.domNode.append(widget.domNode);
	}

	public removeWidget(widget: IContentWidget): void {
		if (this.widgets.get(widget.id) !== widget) {
			return;
		}
		this.widgets.delete(widget.id);
		widget.domNode.remove();
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			for (const widget of this.widgets.values()) {
				widget.domNode.hidden = true;
			}
			return;
		}
		for (const widget of this.widgets.values()) {
			const placement = widget.getPosition();
			const visualLineIndex = placement ? overlay.visualLineProjection.visualLineIndexAt(placement.position) : -1;
			const visualLine = visualLineIndex >= context.layout.renderLines.startLineIndex && visualLineIndex < context.layout.renderLines.endLineIndexExclusive
				? overlay.visualLineProjection.lineAt(visualLineIndex)
				: undefined;
			widget.domNode.hidden = !placement || !visualLine;
			if (!placement || !visualLine) {
				continue;
			}
			widget.domNode.style.position = 'absolute';
			const lineText = overlay.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, placement.position.columnIndex);
			const left = overlay.textLeft + (visualLine.wrappedTextIndentWidth ?? 0) + overlay.textMeasurer.measureLineWidth(lineText);
			widget.domNode.style.left = `${left}px`;
			const lineTop = context.viewportData.getLineTop(visualLineIndex);
			const preference = placement?.preference ?? 'below';
			const top = preference === 'above'
				? lineTop - widget.domNode.offsetHeight
				: preference === 'exact' ? lineTop : lineTop + context.layout.lineHeight;
			widget.domNode.style.top = `${top}px`;
		}
	}
}
