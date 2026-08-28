import { h } from '../../../../base/browser/dom.js';

/** Clipboard data readable by an editor input adapter. */
export interface IReadableClipboardData {
	readonly types: readonly string[];
	readonly files: readonly File[];
	getData(type: string): string;
}

/** Clipboard data writable by an editor input adapter. */
export interface IWritableClipboardData {
	setData(type: string, value: string): void;
}

/** A copy or cut event exposed before the clipboard contribution handles it. */
export interface IClipboardCopyEvent {
	readonly isCut: boolean;
	readonly clipboardData: IWritableClipboardData;
	readonly hasClipboardData: boolean;
	readonly browserEvent: ClipboardEvent;
	setHandled(): void;
	readonly isHandled: boolean;
}

/** A paste event exposed before the clipboard contribution handles it. */
export interface IClipboardPasteEvent {
	readonly clipboardData: IReadableClipboardData;
	readonly text: string;
	readonly browserEvent: ClipboardEvent;
	setHandled(): void;
	readonly isHandled: boolean;
}

export function createClipboardCopyEvent(browserEvent: ClipboardEvent, isCut: boolean): IClipboardCopyEvent {
	let handled = false;
	return {
		isCut,
		clipboardData: createWritableClipboardData(browserEvent.clipboardData),
		hasClipboardData: browserEvent.clipboardData != null,
		browserEvent,
		setHandled: () => {
			if (handled) return;
			handled = true;
			browserEvent.preventDefault();
			browserEvent.stopImmediatePropagation();
		},
		get isHandled(): boolean {
			return handled;
		},
	};
}

export function createClipboardPasteEvent(browserEvent: ClipboardEvent): IClipboardPasteEvent {
	let handled = false;
	const clipboardData = createReadableClipboardData(browserEvent.clipboardData);
	return {
		clipboardData,
		text: readPlainText(clipboardData),
		browserEvent,
		setHandled: () => {
			if (handled) return;
			handled = true;
			browserEvent.preventDefault();
			browserEvent.stopImmediatePropagation();
		},
		get isHandled(): boolean {
			return handled;
		},
	};
}

export function createReadableClipboardData(dataTransfer: DataTransfer | null | undefined): IReadableClipboardData {
	return {
		types: Object.freeze(Array.from(dataTransfer?.types ?? [])),
		files: Object.freeze(Array.from(dataTransfer?.files ?? [])),
		getData: (type: string): string => {
			try {
				return dataTransfer?.getData(type) ?? '';
			} catch {
				return '';
			}
		},
	};
}

export function createWritableClipboardData(dataTransfer: DataTransfer | null | undefined): IWritableClipboardData {
	return {
		setData: (type: string, value: string): void => {
			dataTransfer?.setData(type, value);
		},
	};
}

function readPlainText(clipboardData: IReadableClipboardData): string {
	return clipboardData.getData('text/plain');
}

export function readEditorClipboardText(clipboardData: IReadableClipboardData, ownerDocument: Document): string {
	try {
		const text = clipboardData.getData('text/plain');
		if (text.length > 0) return text;
	} catch {
		// A browser transfer may expose HTML without allowing plain-text access.
	}
	try {
		return readEditorHtmlText(clipboardData.getData('text/html'), ownerDocument);
	} catch {
		return '';
	}
}

/** Reduces untrusted HTML to inert deterministic text for paste and drop paths. */
export function readEditorHtmlText(html: string, ownerDocument: Document): string {
	if (html.length === 0) return '';
	const template = h(ownerDocument, 'template');
	template.innerHTML = html;
	const parts: string[] = [];
	appendHtmlClipboardText(template.content, parts);
	return parts.join('').replaceAll('\u00a0', ' ').replace(/\n{3,}/g, '\n\n').replace(/^\n|\n$/g, '');
}

function appendHtmlClipboardText(node: Node, parts: string[]): void {
	if (node.nodeType === node.TEXT_NODE) {
		parts.push(node.textContent ?? '');
		return;
	}
	if (node.nodeType !== node.ELEMENT_NODE && node.nodeType !== node.DOCUMENT_FRAGMENT_NODE) return;
	const element = node.nodeType === node.ELEMENT_NODE ? node as HTMLElement : undefined;
	if (element && (element.localName === 'script' || element.localName === 'style' || element.localName === 'noscript')) return;
	if (element?.localName === 'br') {
		appendLineBreak(parts);
		return;
	}
	const block = element !== undefined && HTML_CLIPBOARD_BLOCK_ELEMENTS.has(element.localName);
	if (block) appendLineBreak(parts);
	for (const child of node.childNodes) appendHtmlClipboardText(child, parts);
	if (block) appendLineBreak(parts);
}

function appendLineBreak(parts: string[]): void {
	if (parts.length === 0 || parts.at(-1) !== '\n') parts.push('\n');
}

const HTML_CLIPBOARD_BLOCK_ELEMENTS = new Set([
	'address', 'article', 'aside', 'blockquote', 'div', 'dl', 'dt', 'dd', 'fieldset', 'figcaption',
	'figure', 'footer', 'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'header', 'hr', 'li',
	'main', 'nav', 'ol', 'p', 'section', 'table', 'tbody', 'td', 'tfoot', 'th', 'thead', 'tr', 'ul',
]);
