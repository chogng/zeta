import { h, isHTMLElement } from '../../base/browser/dom.js';
import { type IDimension } from '../../base/browser/geometry.js';
import { FastDomNode } from '../../base/browser/fastDomNode.js';
import { Disposable, toDisposable } from '../../base/common/lifecycle.js';

export interface EditorDomOptions {
	readonly rootClassName: string;
	readonly contentClassName: string;
}

/** Owns the stable DOM roots shared by a browser editor projection. */
export class EditorDom extends Disposable {
	private readonly options: EditorDomOptions;
	private domNodeHandle: FastDomNode<HTMLDivElement> | undefined;
	private contentDomNodeHandle: FastDomNode<HTMLDivElement> | undefined;
	private attached = false;

	public constructor(options: EditorDomOptions) {
		super();
		if (!options?.rootClassName?.trim() || !options.contentClassName?.trim()) {
			this.dispose();
			throw new TypeError('Editor DOM requires root and content class names');
		}
		this.options = options;
	}

	public get domNode(): HTMLDivElement {
		return this.requireHandles().domNode.domNode;
	}

	public get contentDomNode(): HTMLDivElement {
		return this.requireHandles().contentDomNode.domNode;
	}

	public attach(parent: HTMLElement): void {
		this.assertNotDisposed();
		if (!isHTMLElement(parent)) throw new TypeError('Editor DOM parent must be an HTMLElement');
		if (this.attached) throw new ReferenceError('Editor DOM has already been attached');
		const domNode = new FastDomNode(h(parent.ownerDocument, 'div', { className: this.options.rootClassName }));
		const contentDomNode = new FastDomNode(h(parent.ownerDocument, 'div', { className: this.options.contentClassName }));
		this.domNodeHandle = domNode;
		this.contentDomNodeHandle = contentDomNode;
		this._register(toDisposable(() => domNode.domNode.remove()));
		parent.append(domNode.domNode);
		this.attached = true;
	}

	public layout(dimension: IDimension): void {
		const { domNode, contentDomNode } = this.requireHandles();
		const width = Math.max(0, dimension.width);
		const height = Math.max(0, dimension.height);
		domNode.setWidth(width);
		domNode.setHeight(height);
		contentDomNode.setWidth(width);
		contentDomNode.setHeight(height);
	}

	private requireHandles(): { readonly domNode: FastDomNode<HTMLDivElement>; readonly contentDomNode: FastDomNode<HTMLDivElement> } {
		if (!this.domNodeHandle || !this.contentDomNodeHandle) throw new ReferenceError('Editor DOM has not been attached');
		return { domNode: this.domNodeHandle, contentDomNode: this.contentDomNodeHandle };
	}
}
