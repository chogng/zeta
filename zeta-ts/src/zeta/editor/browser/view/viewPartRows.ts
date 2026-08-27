import { fragment as createFragment, h, reset } from '../../../base/browser/dom.js';
import { FastDomNode } from '../../../base/browser/fastDomNode.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { type EditorRenderingContext } from './renderingContext.js';

export class ViewPartRows extends Disposable {
	public readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private rows = new Map<number, FastDomNode<HTMLDivElement>>();

	constructor(host: HTMLElement, className: string, private readonly rowClassName: string) {
		super();
		const domNode = h(host.ownerDocument, 'div');
		this.domNode = domNode;
		this.root = new FastDomNode(domNode);
		this.root.setClassName(`stanza-editor-row-layer ${className}`);
		this.domNode.setAttribute('role', 'presentation');
		this.domNode.setAttribute('aria-hidden', 'true');
		this._register(toDisposable(() => this.domNode.remove()));
	}

	public render(context: EditorRenderingContext): ReadonlyMap<number, HTMLElement> {
		const fragment = createFragment(this.domNode.ownerDocument);
		const next = new Map<number, FastDomNode<HTMLDivElement>>();
		const projected = new Map<number, HTMLElement>();
		this.root.setTop(context.layout.renderTop);
		for (let lineIndex = context.layout.renderLines.startLineIndex; lineIndex < context.layout.renderLines.endLineIndexExclusive; lineIndex += 1) {
			let row = this.rows.get(lineIndex);
			if (!row) {
				const element = h(this.domNode.ownerDocument, 'div');
				element.className = this.rowClassName;
				element.dataset.lineIndex = String(lineIndex);
				row = new FastDomNode(element);
			}
			row.setHeight(context.layout.lineHeight);
			row.setLineHeight(context.layout.lineHeight);
			next.set(lineIndex, row);
			projected.set(lineIndex, row.domNode);
			fragment.append(row.domNode);
		}
		reset(this.domNode, fragment);
		this.rows = next;
		return projected;
	}
}
