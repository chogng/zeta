import { h } from '../../../../base/browser/dom.js';
import { TextEditorCursorStyle } from '../../../common/config/editorOptions.js';

export interface ViewCursorOptions {
	readonly style: TextEditorCursorStyle;
	readonly lineWidth: number;
	readonly lineHeight: number;
}

/** Owns one retained caret DOM node. */
export class ViewCursor {
	public readonly domNode: HTMLDivElement;
	private style: TextEditorCursorStyle;

	constructor(host: HTMLElement, selectionIndex: number, private readonly options: ViewCursorOptions) {
		this.style = options.style;
		this.domNode = h(host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-caret';
		this.domNode.dataset.selectionIndex = String(selectionIndex);
		this.domNode.setAttribute('aria-hidden', 'true');
		this.domNode.classList.add(cursorStyleClass(this.style));
	}

	public setStyle(style: TextEditorCursorStyle): void {
		if (style === this.style) return;
		this.domNode.classList.replace(cursorStyleClass(this.style), cursorStyleClass(style));
		this.style = style;
	}

	public render(row: HTMLElement, caretLeft: number, characterLeft: number, characterWidth: number, character: string, rowHeight: number, isPrimary: boolean): void {
		this.domNode.classList.toggle('primary', isPrimary);
		this.domNode.textContent = this.style === TextEditorCursorStyle.Block ? character : '';
		this.domNode.style.left = `${cursorLeft(this.style, caretLeft, characterLeft)}px`;
		this.domNode.style.width = `${cursorWidth(this.style, this.options.lineWidth, characterWidth)}px`;
		const height = cursorHeight(this.style, this.options.lineHeight, rowHeight);
		this.domNode.style.height = `${height}px`;
		this.domNode.style.lineHeight = `${height}px`;
		this.domNode.style.top = `${cursorTop(this.style, rowHeight, height)}px`;
		row.append(this.domNode);
	}
}

function cursorLeft(style: TextEditorCursorStyle, caretLeft: number, characterLeft: number): number {
	return style === TextEditorCursorStyle.Line || style === TextEditorCursorStyle.LineThin ? caretLeft : characterLeft;
}

function cursorStyleClass(style: TextEditorCursorStyle): string {
	switch (style) {
		case TextEditorCursorStyle.Block: return 'cursor-style-block';
		case TextEditorCursorStyle.Underline: return 'cursor-style-underline';
		case TextEditorCursorStyle.LineThin: return 'cursor-style-line-thin';
		case TextEditorCursorStyle.BlockOutline: return 'cursor-style-block-outline';
		case TextEditorCursorStyle.UnderlineThin: return 'cursor-style-underline-thin';
		default: return 'cursor-style-line';
	}
}

function cursorWidth(style: TextEditorCursorStyle, lineWidth: number, characterWidth: number): number {
	if (style === TextEditorCursorStyle.Line) return lineWidth > 0 ? lineWidth : 2;
	if (style === TextEditorCursorStyle.LineThin) return 1;
	return Math.max(1, characterWidth);
}

function cursorHeight(style: TextEditorCursorStyle, lineHeight: number, rowHeight: number): number {
	if (style === TextEditorCursorStyle.Underline) return 2;
	if (style === TextEditorCursorStyle.UnderlineThin) return 2;
	if (style === TextEditorCursorStyle.Line || style === TextEditorCursorStyle.LineThin) {
		return lineHeight > 0 ? Math.min(lineHeight, rowHeight) : rowHeight;
	}
	return rowHeight;
}

function cursorTop(style: TextEditorCursorStyle, rowHeight: number, height: number): number {
	if (style === TextEditorCursorStyle.Underline || style === TextEditorCursorStyle.UnderlineThin) return rowHeight - height;
	return (rowHeight - height) / 2;
}
