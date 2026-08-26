import "./scrollDecoration.css";
import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

/** Projects scroll shadows without owning the editor's scroll state. */
export class ScrollDecorationPart extends EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly topShadow: FastDomNode<HTMLDivElement>;
	private readonly bottomShadow: FastDomNode<HTMLDivElement>;

	constructor(host: HTMLElement) {
		super();
		const ownerDocument = host.ownerDocument;
		this.domNode = this.adopt(h(ownerDocument, "div"), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.topShadow = new FastDomNode(h(ownerDocument, "div"));
		this.bottomShadow = new FastDomNode(h(ownerDocument, "div"));
		this.root.setClassName("stanza-editor-scroll-decoration");
		this.domNode.setAttribute("aria-hidden", "true");
		this.topShadow.setClassName("stanza-editor-scroll-decoration-shadow top");
		this.bottomShadow.setClassName("stanza-editor-scroll-decoration-shadow bottom");
		this.domNode.append(this.topShadow.domNode, this.bottomShadow.domNode);
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		this.root.setWidth(layout.viewportSize.width);
		this.root.setHeight(layout.viewportSize.height);
		this.root.setTransform(`translate3d(${layout.scrollPosition.left}px, ${layout.scrollPosition.top}px, 0)`);
		this.topShadow.setClassName(layout.scrollPosition.top > 0
			? "stanza-editor-scroll-decoration-shadow top visible"
			: "stanza-editor-scroll-decoration-shadow top");
		this.bottomShadow.setClassName(layout.scrollPosition.top < layout.maximumScrollPosition.top
			? "stanza-editor-scroll-decoration-shadow bottom visible"
			: "stanza-editor-scroll-decoration-shadow bottom");
	}
}
