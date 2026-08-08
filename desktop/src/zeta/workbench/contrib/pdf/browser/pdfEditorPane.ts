import type { IDimension } from "../../../../base/browser/geometry.js";
import { throwIfCancelled } from "../../../../base/common/cancellation.js";
import { assertDefined } from "../../../../base/common/types.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { EditorInput } from "../../../browser/parts/editor/editorInput.js";
import { EditorPaneMatch, EditorPaneVisibility, type IEditorPane } from "../../../browser/parts/editor/editorPane.js";
import { matchPdfEditor, PDF_CONTENT_TYPE, PDF_EDITOR_ID } from "./pdfEditorInput.js";
import type { IPdfDocumentLoader } from "./pdfDocumentLoader.js";

/** Browser object-URL operations isolated for deterministic PDF pane cleanup tests. */
export interface IPdfObjectUrlFactory {
  create(bytes: Uint8Array): string;
  revoke(url: string): void;
}

/** Creates PDF object URLs in the same browser realm that owns the editor pane. */
export function createPdfObjectUrlFactory(ownerDocument: Document): IPdfObjectUrlFactory {
  const ownerWindow = ownerDocument.defaultView;
  if (!ownerWindow?.URL.createObjectURL) {
    throw new Error("The current browser does not support PDF object URLs");
  }
  return {
    create: (bytes) => {
      const copy = new ArrayBuffer(bytes.byteLength);
      new Uint8Array(copy).set(bytes);
      return ownerWindow.URL.createObjectURL(new ownerWindow.Blob([copy], { type: PDF_CONTENT_TYPE }));
    },
    revoke: (url) => ownerWindow.URL.revokeObjectURL(url),
  };
}

/** Workbench pane that delegates PDF rendering and document controls to Chromium's native viewer. */
export class PdfEditorPane extends DisposableOwner implements IEditorPane {
  readonly id = PDF_EDITOR_ID;

  private container: HTMLDivElement | undefined;
  private frame: HTMLIFrameElement | undefined;
  private objectUrl: string | undefined;

  constructor(
    private readonly documentLoader: IPdfDocumentLoader,
    private readonly objectUrls: IPdfObjectUrlFactory,
  ) {
    super();
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("PDF editor pane has already been created");
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-pdf-editor";
    container.setAttribute("role", "region");
    container.setAttribute("aria-label", "PDF reader");
    const frame = parent.ownerDocument.createElement("iframe");
    frame.className = "zeta-pdf-editor-frame";
    frame.title = "PDF reader";
    container.append(frame);
    parent.append(container);
    this.container = container;
    this.frame = frame;
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    if (matchPdfEditor(input) === EditorPaneMatch.None) {
      throw new RangeError(`PDF editor cannot open ${input.resource}`);
    }
    const frame = this.requireFrame();
    const bytes = await this.documentLoader.load(input, signal);
    throwIfCancelled(signal, "PDF document loading was cancelled");
    const objectUrl = this.objectUrls.create(bytes);
    if (signal.aborted) {
      this.objectUrls.revoke(objectUrl);
      throwIfCancelled(signal, "PDF document loading was cancelled");
    }
    this.releaseObjectUrl();
    this.objectUrl = objectUrl;
    frame.src = objectUrl;
    frame.title = `${editorLabel(input)} PDF reader`;
  }

  clearInput(): void {
    this.releaseObjectUrl();
    const frame = this.frame;
    if (frame) frame.src = "about:blank";
  }

  layout(_dimension: IDimension): void {}

  setVisible(visibility: EditorPaneVisibility): void {
    if (this.container) this.container.hidden = visibility === EditorPaneVisibility.Hidden;
  }

  focus(): void {
    this.requireFrame().focus();
  }

  override dispose(): void {
    this.clearInput();
    this.container?.remove();
    this.container = undefined;
    this.frame = undefined;
    super.dispose();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  private requireFrame(): HTMLIFrameElement {
    const frame = this.frame;
    assertDefined(frame, new ReferenceError("PDF editor pane has not been created"));
    return frame;
  }

  private releaseObjectUrl(): void {
    if (!this.objectUrl) return;
    this.objectUrls.revoke(this.objectUrl);
    this.objectUrl = undefined;
  }
}

function editorLabel(input: EditorInput): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path).replace(/\/+$/, "");
  return path.slice(path.lastIndexOf("/") + 1) || input.resource.toString();
}
