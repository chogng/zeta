import { computeScreenAwareSize, h } from '../../../../base/browser/dom.js';
import { TextEditorCursorStyle } from '../../../common/config/editorOptions.js';

export interface ViewCursorOptions {
	readonly style: TextEditorCursorStyle;
	readonly lineWidth: number;
	readonly lineHeight: number;
}

export type ViewCursorPlurality = 'single' | 'primary' | 'secondary';

export interface ViewCursorCharacterPresentation {
	readonly classNames: readonly string[];
	readonly fontStyle?: string;
	readonly fontWeight?: string;
	readonly textDecorationLine?: string;
}

export interface ViewCursorRenderInput {
	readonly top: number;
	readonly caretLeft: number;
	readonly characterLeft: number;
	readonly characterWidth: number;
	readonly character: string;
	readonly rowHeight: number;
	readonly plurality: ViewCursorPlurality;
	readonly pauseMovementAnimation: boolean;
	readonly presentation?: ViewCursorCharacterPresentation;
}

/** Owns one retained caret DOM node. */
export class ViewCursor {
	public readonly domNode: HTMLDivElement;
	private readonly ownerWindow: Window;
	private style: TextEditorCursorStyle;
	private lineWidth: number;

	constructor(host: HTMLElement, selectionIndex: number, private readonly options: ViewCursorOptions) {
		const ownerWindow = host.ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('Editor cursor requires a browser window');
		this.ownerWindow = ownerWindow;
		this.style = options.style;
		this.lineWidth = options.lineWidth;
		this.domNode = h(host.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-caret';
		this.domNode.dataset.selectionIndex = String(selectionIndex);
		this.domNode.setAttribute('aria-hidden', 'true');
		this.domNode.classList.add(cursorStyleClass(this.style));
		host.append(this.domNode);
	}

	public setStyle(style: TextEditorCursorStyle): void {
		if (style === this.style) return;
		this.domNode.classList.replace(cursorStyleClass(this.style), cursorStyleClass(style));
		this.style = style;
	}

	public setLineWidth(lineWidth: number): void {
		this.lineWidth = lineWidth;
	}

	public render(input: ViewCursorRenderInput): void {
		const width = cursorWidth(this.ownerWindow, this.style, this.lineWidth, input.characterWidth);
		let left = cursorLeft(this.style, input.caretLeft, input.characterLeft);
		let paddingLeft = 0;
		if (this.style === TextEditorCursorStyle.Line && width >= 2 && left >= 1) {
			paddingLeft = 1;
			left -= paddingLeft;
		}
		const rendersCharacter = this.style === TextEditorCursorStyle.Block || (this.style === TextEditorCursorStyle.Line && width > 2);
		this.domNode.className = [
			'stanza-editor-caret',
			cursorStyleClass(this.style),
			input.plurality === 'secondary' ? '' : 'primary',
			input.plurality === 'single' ? '' : `cursor-${input.plurality}`,
			...(rendersCharacter ? input.presentation?.classNames ?? [] : []),
		].filter(Boolean).join(' ');
		this.domNode.textContent = rendersCharacter ? input.character : '';
		this.domNode.style.fontStyle = rendersCharacter ? input.presentation?.fontStyle ?? '' : '';
		this.domNode.style.fontWeight = rendersCharacter ? input.presentation?.fontWeight ?? '' : '';
		this.domNode.style.textDecorationLine = rendersCharacter ? input.presentation?.textDecorationLine ?? '' : '';
		this.domNode.style.transitionProperty = input.pauseMovementAnimation ? 'none' : '';
		this.domNode.style.left = `${left}px`;
		this.domNode.style.paddingLeft = `${paddingLeft}px`;
		this.domNode.style.width = `${width}px`;
		const height = cursorHeight(this.style, this.options.lineHeight, input.rowHeight);
		this.domNode.style.height = `${height}px`;
		this.domNode.style.lineHeight = `${height}px`;
		this.domNode.style.top = `${input.top + cursorTop(this.style, input.rowHeight, height)}px`;
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

function cursorWidth(ownerWindow: Window, style: TextEditorCursorStyle, lineWidth: number, characterWidth: number): number {
	if (style === TextEditorCursorStyle.Line) return computeScreenAwareSize(ownerWindow, lineWidth > 0 ? lineWidth : 2);
	if (style === TextEditorCursorStyle.LineThin) return computeScreenAwareSize(ownerWindow, 1);
	return Math.max(1, characterWidth);
}

function cursorHeight(style: TextEditorCursorStyle, lineHeight: number, rowHeight: number): number {
	if (style === TextEditorCursorStyle.Underline) return 2;
	if (style === TextEditorCursorStyle.UnderlineThin) return 1;
	if (style === TextEditorCursorStyle.Line || style === TextEditorCursorStyle.LineThin) {
		return lineHeight > 0 ? Math.min(lineHeight, rowHeight) : rowHeight;
	}
	return rowHeight;
}

function cursorTop(style: TextEditorCursorStyle, rowHeight: number, height: number): number {
	if (style === TextEditorCursorStyle.Underline || style === TextEditorCursorStyle.UnderlineThin) return rowHeight - height;
	return (rowHeight - height) / 2;
}
