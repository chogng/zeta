import "./media/message.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns transient editor-local messages without replacing host notifications. */
export class MessageController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly viewport: EditorViewport) {
    super();
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-editor-message";
    this.element.hidden = true;
    this.element.setAttribute("role", "status");
    this.element.setAttribute("aria-live", "polite");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.defer(() => { if (this.timer) clearTimeout(this.timer); });
  }

  show(message: string, durationMs = 3000): void {
    if (typeof message !== "string" || message.trim().length === 0) throw new TypeError("Alpha editor message must be non-empty");
    if (!Number.isSafeInteger(durationMs) || durationMs < 0) throw new RangeError("Alpha editor message duration must be non-negative");
    if (this.timer) clearTimeout(this.timer);
    this.element.textContent = message.trim();
    this.element.hidden = false;
    if (durationMs > 0) this.timer = setTimeout(() => { this.element.hidden = true; this.timer = undefined; }, durationMs);
  }

  hide(): void { if (this.timer) clearTimeout(this.timer); this.timer = undefined; this.element.hidden = true; }
}
