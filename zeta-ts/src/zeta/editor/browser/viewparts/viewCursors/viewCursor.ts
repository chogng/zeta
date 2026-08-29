import { h } from '../../../../base/browser/dom.js';
import { TextEditorCursorBlinkingStyle, TextEditorCursorStyle } from '../../../common/config/editorOptions.js';

export interface ViewCursorOptions {
	readonly style: TextEditorCursorStyle;
	readonly blinking: TextEditorCursorBlinkingStyle;
	readonly lineWidth: number;
	readonly lineHeight: number;
}

/** Owns one retained caret DOM node. */
export class ViewCursor {
	public readonly domNode: HTMLDivElement;

	constructor(host: HTMLElement, selectionIndex: number, private readonly options: ViewCursorOptions) {
		this.domNode = h(host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-caret';
		this.domNode.dataset.selectionIndex = String(selectionIndex);
		this.domNode.classList.add(cursorStyleClass(options.style), cursorBlinkingClass(options.blinking));
	}

	public render(row: HTMLElement, left: number, characterWidth: number, rowHeight: number, isPrimary: boolean): void {
		this.domNode.classList.toggle('primary', isPrimary);
		this.domNode.style.left = `${left}px`;
		this.domNode.style.width = `${cursorWidth(this.options, characterWidth)}px`;
		const height = cursorHeight(this.options, rowHeight);
		this.domNode.style.height = `${height}px`;
		this.domNode.style.top = `${cursorTop(this.options.style, rowHeight, height)}px`;
		row.append(this.domNode);
	}
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

function cursorBlinkingClass(blinking: TextEditorCursorBlinkingStyle): string {
	switch (blinking) {
		case TextEditorCursorBlinkingStyle.Smooth: return 'cursor-blinking-smooth';
		case TextEditorCursorBlinkingStyle.Phase: return 'cursor-blinking-phase';
		case TextEditorCursorBlinkingStyle.Expand: return 'cursor-blinking-expand';
		case TextEditorCursorBlinkingStyle.Solid: return 'cursor-blinking-solid';
		case TextEditorCursorBlinkingStyle.Hidden: return 'cursor-blinking-hidden';
		default: return 'cursor-blinking-blink';
	}
}

function cursorWidth(options: ViewCursorOptions, characterWidth: number): number {
	if (options.style === TextEditorCursorStyle.Line) return options.lineWidth > 0 ? options.lineWidth : 2;
	if (options.style === TextEditorCursorStyle.LineThin) return 1;
	return Math.max(1, characterWidth);
}

function cursorHeight(options: ViewCursorOptions, rowHeight: number): number {
	if (options.style === TextEditorCursorStyle.Underline) return 2;
	if (options.style === TextEditorCursorStyle.UnderlineThin) return 1;
	return options.lineHeight > 0 ? Math.min(options.lineHeight, rowHeight) : rowHeight;
}

function cursorTop(style: TextEditorCursorStyle, rowHeight: number, height: number): number {
	if (style === TextEditorCursorStyle.Underline || style === TextEditorCursorStyle.UnderlineThin) return rowHeight - height;
	return (rowHeight - height) / 2;
}
