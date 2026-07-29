import MarkdownIt from "markdown-it";
import {
  isSafeMarkdownLink,
  sanitizeMarkdownHtmlToString,
} from "../../../base/browser/markdownRenderer.js";
import {
  Emitter,
  type Event,
} from "../../../base/common/event.js";
import {
  DisposableOwner,
} from "../../../base/common/lifecycle.js";
import {
  WebviewElement,
} from "../../webview/browser/webviewElement.js";

export interface MarkdownPreviewOptions {
  readonly ownerDocument: Document;
  readonly markdown?: string;
  readonly title?: string;
}

interface OpenLinkMessage {
  readonly type: "openLink";
  readonly href: string;
}

const markdownParser = new MarkdownIt({
  breaks: true,
  html: true,
  linkify: true,
});
const MAX_MARKDOWN_LENGTH = 4 * 1024 * 1024;

const PREVIEW_STYLE = `
:root {
  --zeta-font-family-monospace:
    ui-monospace,
    "SFMono-Regular",
    Menlo,
    Monaco,
    Consolas,
    "Liberation Mono",
    "Courier New",
    monospace;

  color-scheme: light dark;
  font: 14px/1.6 system-ui, sans-serif;
}
body {
  box-sizing: border-box;
  color: CanvasText;
  background: Canvas;
  margin: 0 auto;
  max-width: 920px;
  padding: 24px 32px 64px;
  overflow-wrap: anywhere;
}
h1, h2, h3, h4, h5, h6 { line-height: 1.25; margin: 1.35em 0 0.55em; }
h1, h2 { border-bottom: 1px solid GrayText; padding-bottom: 0.25em; }
p, blockquote, pre, table, ul, ol { margin: 0.85em 0; }
blockquote { border-left: 3px solid GrayText; margin-left: 0; padding-left: 1em; }
code, pre { font-family: var(--zeta-font-family-monospace); }
code { background: color-mix(in srgb, CanvasText 10%, Canvas); border-radius: 3px; padding: 0.12em 0.3em; }
pre { background: color-mix(in srgb, CanvasText 8%, Canvas); overflow: auto; padding: 1em; }
pre code { background: none; padding: 0; }
a { color: LinkText; cursor: pointer; }
img { max-width: 100%; }
table { border-collapse: collapse; display: block; max-width: 100%; overflow: auto; }
th, td { border: 1px solid GrayText; padding: 0.35em 0.7em; }
input[type="checkbox"] { margin: 0 0.35em 0 0; }
`;

const LINK_BRIDGE_SCRIPT = `
(() => {
  const api = acquireZetaWebviewApi();
  document.addEventListener("click", (event) => {
    const anchor = event.target instanceof Element
      ? event.target.closest("a[href]")
      : null;
    if (!anchor) return;
    event.preventDefault();
    api.postMessage({
      type: "openLink",
      href: anchor.getAttribute("href")
    });
  });
})();
`;

/**
 * Renders a full Markdown document inside the opaque-origin iframe boundary.
 */
export class MarkdownPreview extends DisposableOwner {
  readonly #ownerDocument: Document;
  readonly #webview: WebviewElement;
  readonly #onDidOpenLink = this.own(new Emitter<string>());
  #active = true;

  readonly element: HTMLIFrameElement;
  readonly onDidOpenLink: Event<string> = this.#onDidOpenLink.event;

  constructor(options: MarkdownPreviewOptions) {
    super();
    this.#ownerDocument = options.ownerDocument;
    this.#webview = this.own(new WebviewElement({
      ownerDocument: options.ownerDocument,
      title: options.title ?? "Markdown preview",
    }));
    this.element = this.#webview.element;
    this.own(this.#webview.onDidMessage((message) => {
      const openLink = validateOpenLinkMessage(message);
      if (openLink) this.#onDidOpenLink.fire(openLink.href);
    }));
    this.defer(() => {
      this.#active = false;
    });
    this.setMarkdown(options.markdown ?? "");
  }

  setMarkdown(markdown: string): void {
    this.#requireActive();
    if (typeof markdown !== "string") {
      throw new TypeError("Markdown must be a string");
    }
    if (markdown.length > MAX_MARKDOWN_LENGTH) {
      throw new Error("Markdown exceeds the supported size");
    }
    const parserHtml = markdownParser.render(markdown);
    const safeHtml = sanitizeMarkdownHtmlToString({
      ownerDocument: this.#ownerDocument,
    }, parserHtml);
    this.#webview.setHtml(
      `<style>${PREVIEW_STYLE}</style>` +
        `<main class="zeta-markdown-preview">${safeHtml}</main>` +
        `<script>${LINK_BRIDGE_SCRIPT}</script>`,
    );
  }

  focus(): void {
    this.#requireActive();
    this.#webview.focus();
  }

  #requireActive(): void {
    if (!this.#active) {
      throw new ReferenceError("MarkdownPreview is already disposed");
    }
  }
}

function validateOpenLinkMessage(
  message: unknown,
): OpenLinkMessage | undefined {
  if (typeof message !== "object" || message === null) return undefined;
  const candidate = message as Record<string, unknown>;
  if (
    Object.keys(candidate).length !== 2 ||
    candidate.type !== "openLink" ||
    typeof candidate.href !== "string" ||
    !isSafeMarkdownLink(candidate.href)
  ) {
    return undefined;
  }
  return {
    type: "openLink",
    href: candidate.href,
  };
}
