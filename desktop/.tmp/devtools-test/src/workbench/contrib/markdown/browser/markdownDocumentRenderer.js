import { DisposableOwner, } from "../../../../base/common/lifecycle.js";
import { MarkdownPreview, } from "../../../../platform/markdown/browser/index.js";
/**
 * Workbench adapter for a sandboxed Markdown document preview.
 *
 * The platform component owns parsing, sanitization, and iframe isolation;
 * this view owns the product link-opening policy and editor-compatible shape.
 */
export class MarkdownDocumentView extends DisposableOwner {
    #preview;
    #openLink;
    #active = true;
    element;
    constructor(options) {
        super();
        this.#openLink = options.openLink;
        this.#preview = this.own(new MarkdownPreview({
            ownerDocument: options.ownerDocument,
            markdown: options.markdown,
            title: options.title,
        }));
        this.element = this.#preview.element;
        this.element.classList.add("zeta-markdown-document-view");
        this.own(this.#preview.onDidOpenLink((href) => {
            void Promise.resolve(this.#openLink(href)).catch((error) => {
                console.error("Unable to open Markdown link", error);
            });
        }));
        this.defer(() => {
            this.#active = false;
        });
    }
    setMarkdown(markdown) {
        this.#requireActive();
        this.#preview.setMarkdown(markdown);
    }
    focus() {
        this.#requireActive();
        this.#preview.focus();
    }
    #requireActive() {
        if (!this.#active) {
            throw new ReferenceError("MarkdownDocumentView is already disposed");
        }
    }
}
