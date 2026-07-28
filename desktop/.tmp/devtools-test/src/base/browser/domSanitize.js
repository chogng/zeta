import createDOMPurify from "dompurify";
/**
 * Sanitizes untrusted HTML using a DOMPurify instance bound to its destination
 * document.
 *
 * Callers own the allowlist and any post-attribute policy. A new instance is
 * created per operation so hooks cannot leak between consumers or windows.
 */
export function sanitizeHtmlToFragment(rawHtml, options) {
    const ownerWindow = options.ownerDocument.defaultView;
    if (!ownerWindow) {
        throw new Error("HTML sanitization requires a document with a window");
    }
    const purifier = createDOMPurify(ownerWindow);
    if (options.afterSanitizeAttributes) {
        const callback = options.afterSanitizeAttributes;
        purifier.addHook("afterSanitizeAttributes", (element) => callback(element));
    }
    return purifier.sanitize(rawHtml, {
        ...options.config,
        RETURN_DOM_FRAGMENT: true,
    });
}
