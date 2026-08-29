import { h } from '../../../../base/browser/dom.js';

/** Owns one retained caret DOM node. */
export class ViewCursor {
	public readonly domNode: HTMLDivElement;

	constructor(host: HTMLElement, selectionIndex: number) {
		this.domNode = h(host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-caret';
		this.domNode.dataset.selectionIndex = String(selectionIndex);
	}

	public render(row: HTMLElement, left: number, isPrimary: boolean): void {
		this.domNode.classList.toggle('primary', isPrimary);
		this.domNode.style.left = `${left}px`;
		row.append(this.domNode);
	}
}
