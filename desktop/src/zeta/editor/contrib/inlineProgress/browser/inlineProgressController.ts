import "./media/inlineProgress.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Provides a reusable inline progress presentation for asynchronous editor requests. */
export class InlineProgressController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private active = 0;

  constructor(private readonly viewport: EditorViewport) {
    super();
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = "aster-editor-inline-progress";
    this.element.hidden = true;
    this.element.setAttribute("role", "status");
    this.element.setAttribute("aria-live", "polite");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
  }

  async run<T>(label: string, task: Promise<T>): Promise<T> {
    if (typeof label !== "string" || label.trim().length === 0) throw new TypeError("Aster inline progress label must be non-empty");
    const token = ++this.active;
    this.element.textContent = label.trim();
    this.element.hidden = false;
    try { return await task; } finally { if (token === this.active) this.element.hidden = true; }
  }
}
