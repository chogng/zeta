/** A permission-gated Async Clipboard item reduced to Stanza's safe text formats. */
export interface ClipboardRichTextItem {
	readonly plainText?: string;
	readonly html?: string;
}

/** Reads rich text only after a native paste event cannot expose a transferable payload. */
export interface ClipboardRichTextReader {
	readText(): PromiseLike<ClipboardRichTextItem | undefined>;
}

/** Writes Stanza's portable plain-text and safe HTML clipboard representations. */
export interface ClipboardRichTextWriter {
	writeText(item: Required<ClipboardRichTextItem>): PromiseLike<void>;
}

/** Creates the browser adapter without exposing ClipboardItem or Blob to Stanza consumers. */
export function createStanzaBrowserClipboardRichTextReader(ownerDocument: Document): ClipboardRichTextReader | undefined {
	const clipboard = ownerDocument.defaultView?.navigator.clipboard;
	if (!clipboard || typeof clipboard.read !== "function") return undefined;
	return Object.freeze({
		readText: async (): Promise<ClipboardRichTextItem | undefined> => {
			const items = await clipboard.read();
			for (const item of items) {
				const plainText = item.types.includes("text/plain") ? await (await item.getType("text/plain")).text() : undefined;
				const html = item.types.includes("text/html") ? await (await item.getType("text/html")).text() : undefined;
				if (plainText !== undefined || html !== undefined) return Object.freeze({
					...(plainText === undefined ? {} : { plainText }),
					...(html === undefined ? {} : { html }),
				});
			}
			return undefined;
		},
	});
}

/**
 * Creates a permission-gated Async Clipboard writer when this browser exposes
 * both `Clipboard.write` and the `ClipboardItem` constructor.
 */
export function createStanzaBrowserClipboardRichTextWriter(ownerDocument: Document): ClipboardRichTextWriter | undefined {
	const ownerWindow = ownerDocument.defaultView;
	const clipboard = ownerWindow?.navigator.clipboard;
	const ClipboardItemConstructor = ownerWindow?.ClipboardItem;
	const BlobConstructor = ownerWindow?.Blob;
	if (!clipboard || typeof clipboard.write !== "function" || !ClipboardItemConstructor || !BlobConstructor) return undefined;
	return Object.freeze({
		writeText: (item: Required<ClipboardRichTextItem>): Promise<void> => clipboard.write([new ClipboardItemConstructor({
			"text/plain": new BlobConstructor([item.plainText], { type: "text/plain" }),
			"text/html": new BlobConstructor([item.html], { type: "text/html" }),
		})]),
	});
}
