import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { type EditorLineGutterDecoration } from "../margin/lineGutterDecoration.js";

/** DOM nodes shared by the line renderer and row-level visual parts. */
export interface RenderedLine {
	readonly domNode: FastDomNode<HTMLDivElement>;
	readonly numberDomNode: FastDomNode<HTMLSpanElement>;
	readonly featureGutterElement: HTMLElement;
	readonly diagnosticDomNode: FastDomNode<HTMLSpanElement>;
	readonly textElement: HTMLSpanElement;
	readonly indentationElement: HTMLDivElement;
	readonly decorationElement: HTMLDivElement;
	readonly linesDecorationElement: HTMLDivElement;
	readonly selectionElement: HTMLDivElement;
	readonly cursorElement: HTMLDivElement;
	readonly compositionElement: HTMLDivElement;
}

/** Creates one reusable virtual-line DOM subtree owned by the ViewLines part. */
export function createStanzaRenderedLine(ownerDocument: Document, lineIndex: number, gutterDecoration: EditorLineGutterDecoration | undefined): RenderedLine {
	const domNode = new FastDomNode(h(ownerDocument, "div"));
	const numberDomNode = new FastDomNode(h(ownerDocument, "span"));
	const featureGutterElement = gutterDecoration?.create(ownerDocument) ?? h(ownerDocument, "span");
	if (!gutterDecoration) featureGutterElement.hidden = true;
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
	linesDecorationElement.setAttribute("aria-hidden", "true");
	selectionElement.className = "stanza-editor-line-selections";
	selectionElement.setAttribute("aria-hidden", "true");
	cursorElement.className = "stanza-editor-line-cursors";
	cursorElement.setAttribute("aria-hidden", "true");
	compositionElement.className = "stanza-editor-line-composition";
	compositionElement.setAttribute("aria-hidden", "true");
	domNode.domNode.append(indentationElement, decorationElement, linesDecorationElement, selectionElement, cursorElement, compositionElement, featureGutterElement, diagnosticDomNode.domNode, numberDomNode.domNode, textElement);
	return {
		domNode,
		numberDomNode,
		featureGutterElement,
		diagnosticDomNode,
		textElement,
		indentationElement,
		decorationElement,
		linesDecorationElement,
		selectionElement,
		cursorElement,
		compositionElement,
	};
}
