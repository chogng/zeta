import {
  DisposableOwner,
} from "../../../../base/common/lifecycle.js";
import {
  MarkdownPreview,
} from "../../../../platform/markdown/browser/index.js";

export interface MarkdownDocumentViewOptions {
  readonly ownerDocument: Document;
  readonly markdown?: string;
  readonly title?: string;
  readonly openLink: (href: string) => void | Promise<void>;
}

/**
 * Workbench adapter for a sandboxed Markdown document preview.
 *
 * The platform component owns parsing, sanitization, and iframe isolation;
 * this view owns the product link-opening policy and editor-compatible shape.
 */
export class MarkdownDocumentView extends DisposableOwner {
  private readonly preview: MarkdownPreview;
  private readonly openLink: (href: string) => void | Promise<void>;
  private active = true;

  readonly element: HTMLIFrameElement;

  constructor(options: MarkdownDocumentViewOptions) {
    super();
    this.openLink = options.openLink;
    this.preview = this.own(new MarkdownPreview({
      ownerDocument: options.ownerDocument,
      markdown: options.markdown,
      title: options.title,
    }));
    this.element = this.preview.element;
    this.element.classList.add("zeta-markdown-document-view");
    this.own(this.preview.onDidOpenLink((href) => {
      void Promise.resolve(this.openLink(href)).catch((error: unknown) => {
        console.error("Unable to open Markdown link", error);
      });
    }));
    this.defer(() => {
      this.active = false;
    });
  }

  setMarkdown(markdown: string): void {
    this.requireActive();
    this.preview.setMarkdown(markdown);
  }

  focus(): void {
    this.requireActive();
    this.preview.focus();
  }

  private requireActive(): void {
    if (!this.active) {
      throw new ReferenceError("MarkdownDocumentView is already disposed");
    }
  }
}
