import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { DomReadingContext } from './domReadingContext.js';
import { RangeUtil } from './rangeUtil.js';
import { ViewLineTextDirection, type ViewLineOptions } from './viewLineOptions.js';
import { DomPosition, type CharacterMapping } from '../../../common/viewLayout/viewLineRenderer.js';
import { type FloatHorizontalRange } from '../../view/renderingContext.js';
import { projectStanzaSemanticTokenLine, type BracketColorizationSpan, type ResolvedSemanticToken } from './semanticTokenPresentation.js';

interface BrowserCaretPosition {
	readonly offsetNode: Node;
	readonly offset: number;
}

interface BrowserCaretRange {
	readonly startContainer: Node;
	readonly startOffset: number;
}

interface BrowserCaretDocument {
	caretPositionFromPoint?(x: number, y: number): BrowserCaretPosition | null;
	caretRangeFromPoint?(x: number, y: number): BrowserCaretRange | null;
}

/** Owns one reusable virtual-line DOM subtree rendered by ViewLines. */
export class ViewLine {
	public readonly domNode: FastDomNode<HTMLDivElement>;
	public readonly textElement: HTMLSpanElement;
	private characterMapping: CharacterMapping;
	private renderedText = '';

	constructor(host: HTMLElement, lineIndex: number, private readonly options: ViewLineOptions) {
		const domNode = new FastDomNode(h(host.ownerDocument, "div"));
		const textElement = h(host.ownerDocument, "span");
		domNode.setClassName("stanza-editor-line");
		domNode.domNode.dataset.lineIndex = String(lineIndex);
		textElement.className = "stanza-editor-line-text";
		textElement.dir = options.textDirection;
		domNode.domNode.append(textElement);
		this.domNode = domNode;
		this.textElement = textElement;
		this.characterMapping = projectStanzaSemanticTokenLine(this.textElement, '', [], [], this.options.tabSize);
	}

	public hasTextOffset(offset: number): boolean {
		return Number.isSafeInteger(offset) && offset >= 0 && offset <= this.renderedText.length;
	}

	public renderText(text: string, tokens: readonly ResolvedSemanticToken[], brackets: readonly BracketColorizationSpan[]): void {
		this.characterMapping = projectStanzaSemanticTokenLine(this.textElement, text, tokens, brackets, this.options.tabSize);
		this.renderedText = text;
	}

	public layoutLine(lineHeight: number): void {
		this.domNode.setHeight(lineHeight);
		this.domNode.setLineHeight(lineHeight);
	}

	public getHorizontalRanges(startOffset: number, endOffset: number, context = this.createReadingContext()): readonly FloatHorizontalRange[] | undefined {
		if (!this.hasTextOffset(startOffset) || !this.hasTextOffset(endOffset) || endOffset < startOffset) {
			throw new RangeError('View line offsets must be ordered UTF-16 positions');
		}
		const start = this.characterMapping.getDomPosition(startOffset + 1);
		const end = this.characterMapping.getDomPosition(endOffset + 1);
		return RangeUtil.readHorizontalRanges(this.textElement, start.partIndex, start.charIndex, end.partIndex, end.charIndex, context) ?? undefined;
	}

	public getCaretLeft(offset: number): number | undefined {
		return this.getHorizontalRanges(offset, offset)?.[0]?.left;
	}

	public isRightToLeft(): boolean {
		if (this.options.textDirection === ViewLineTextDirection.RightToLeft) return true;
		if (this.options.textDirection === ViewLineTextDirection.LeftToRight) return false;
		return this.textElement.ownerDocument.defaultView?.getComputedStyle(this.textElement).direction === 'rtl';
	}

	public getOffsetAtClientPoint(clientX: number, clientY: number): number | undefined {
		if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) throw new RangeError('View line hit-test coordinates must be finite');
		const document = this.textElement.ownerDocument as unknown as BrowserCaretDocument;
		const position = document.caretPositionFromPoint?.(clientX, clientY) ?? document.caretRangeFromPoint?.(clientX, clientY);
		if (!position) return undefined;
		const node = 'offsetNode' in position ? position.offsetNode : position.startContainer;
		const offset = 'offsetNode' in position ? position.offset : position.startOffset;
		if (!this.textElement.contains(node)) return undefined;
		if (node === this.textElement) {
			const spanNode = (this.textElement.children[offset] ?? this.textElement.children[offset - 1]) as HTMLElement | undefined;
			if (!spanNode) return undefined;
			return this.getColumnOfNodeOffset(spanNode, spanNode === this.textElement.children[offset] ? 0 : spanNode.textContent?.length ?? 0);
		}
		const spanNode = node.nodeType === node.TEXT_NODE ? node.parentElement : node as HTMLElement;
		if (!spanNode) return undefined;
		const charOffset = node.nodeType === node.TEXT_NODE ? offset : offset === 0 ? 0 : spanNode.textContent?.length ?? 0;
		return this.getColumnOfNodeOffset(spanNode, charOffset);
	}

	public getColumnOfNodeOffset(spanNode: HTMLElement, offset: number): number | undefined {
		const partIndex = Array.prototype.indexOf.call(this.textElement.children, spanNode) as number;
		if (partIndex < 0 || !Number.isSafeInteger(offset) || offset < 0) return undefined;
		const partLength = spanNode.textContent?.length ?? 0;
		if (offset > partLength) return undefined;
		return this.characterMapping.getColumn(new DomPosition(partIndex, offset), partLength) - 1;
	}

	private createReadingContext(): DomReadingContext {
		return new DomReadingContext(this.domNode.domNode, this.textElement);
	}
}
