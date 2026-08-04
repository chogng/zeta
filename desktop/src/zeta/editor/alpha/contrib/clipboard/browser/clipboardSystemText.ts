/**
 * Reads plain text from the browser's system clipboard after a user-gesture
 * paste event could not expose a textual DataTransfer representation.
 */
export interface AlphaClipboardSystemTextReader {
  readText(): PromiseLike<string>;
}

/**
 * Creates the browser adapter only when the owning window exposes the
 * permission-gated Async Clipboard text read API.
 */
export function createAlphaBrowserClipboardSystemTextReader(ownerDocument: Document): AlphaClipboardSystemTextReader | undefined {
  const clipboard = ownerDocument.defaultView?.navigator.clipboard;
  return clipboard && typeof clipboard.readText === "function"
    ? Object.freeze({ readText: () => clipboard.readText() })
    : undefined;
}
