import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";

/** Owns one reusable virtual-line DOM subtree rendered by ViewLines. */
export class ViewLine {
	public readonly domNode: FastDomNode<HTMLDivElement>;
	public readonly numberDomNode: FastDomNode<HTMLSpanElement>;
	public readonly diagnosticDomNode: FastDomNode<HTMLSpanElement>;
	public readonly textElement: HTMLSpanElement;
	public readonly indentationElement: HTMLDivElement;
	public readonly decorationElement: HTMLDivElement;
	public readonly linesDecorationElement: HTMLDivElement;
	public readonly selectionElement: HTMLDivElement;
	public readonly cursorElement: HTMLDivElement;
	public readonly compositionElement: HTMLDivElement;

	constructor(ownerDocument: Document, lineIndex: number) {
		const domNode = new FastDomNode(h(ownerDocument, "div"));
		const marginElement = h(ownerDocument, "span");
		const numberDomNode = new FastDomNode(h(ownerDocument, "span"));
		const diagnosticDomNode = new FastDomNode(h(ownerDocument, "span"));
		const textElement = h(ownerDocument, "span");
		const indentationElement = h(ownerDocument, "div");
		const decorationElement = h(ownerDocument, "div");
		const linesDecorationElement = h(ownerDocument, "div");
		const selectionElement = h(ownerDocument, "div");
		const cursorElement = h(ownerDocument, "div");
		const compositionElement = h(ownerDocument, "div");
		domNode.setClassName("stanza-editor-line");
		domNode.domNode.dataset.lineIndex = String(lineIndex);
		marginElement.className = "stanza-editor-line-margin";
		marginElement.setAttribute("aria-hidden", "true");
		numberDomNode.setClassName("stanza-editor-line-number");
		numberDomNode.domNode.setAttribute("aria-hidden", "true");
		diagnosticDomNode.setClassName("stanza-editor-diagnostic-marker");
		diagnosticDomNode.setHidden(true);
		diagnosticDomNode.domNode.setAttribute("aria-hidden", "true");
		textElement.className = "stanza-editor-line-text";
		indentationElement.className = "stanza-editor-line-indent-guides";
		indentationElement.setAttribute("aria-hidden", "true");
		decorationElement.className = "stanza-editor-line-decorations";
		decorationElement.setAttribute("aria-hidden", "true");
		linesDecorationElement.className = "stanza-editor-line-lines-decorations";
		selectionElement.className = "stanza-editor-line-selections";
		selectionElement.setAttribute("aria-hidden", "true");
		cursorElement.className = "stanza-editor-line-cursors";
		cursorElement.setAttribute("aria-hidden", "true");
		compositionElement.className = "stanza-editor-line-composition";
		compositionElement.setAttribute("aria-hidden", "true");
		marginElement.append(numberDomNode.domNode);
		domNode.domNode.append(indentationElement, decorationElement, linesDecorationElement, selectionElement, cursorElement, compositionElement, diagnosticDomNode.domNode, marginElement, textElement);
		this.domNode = domNode;
		this.numberDomNode = numberDomNode;
		this.diagnosticDomNode = diagnosticDomNode;
		this.textElement = textElement;
		this.indentationElement = indentationElement;
		this.decorationElement = decorationElement;
		this.linesDecorationElement = linesDecorationElement;
		this.selectionElement = selectionElement;
		this.cursorElement = cursorElement;
		this.compositionElement = compositionElement;
	}
}
