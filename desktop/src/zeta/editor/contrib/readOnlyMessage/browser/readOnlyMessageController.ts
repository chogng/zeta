import "./media/readOnlyMessage.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface ReadOnlyMessageControllerOptions {
  readonly message?: string;
  readonly durationMs?: number;
}

/** Explains blocked mutations without making read-only state part of model policy. */
export class ReadOnlyMessageController extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly durationMs: number;
  private hideTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    options: ReadOnlyMessageControllerOptions = {},
  ) {
    super();
    const message = options.message ?? "This editor is read-only";
    this.durationMs = options.durationMs ?? 2_400;
    if (typeof message !== "string" || message.trim().length === 0) {
      this.dispose();
      throw new TypeError("Aster read-only message must not be empty");
    }
    if (!Number.isSafeInteger(this.durationMs) || this.durationMs < 0) {
      this.dispose();
      throw new RangeError("Aster read-only message duration must be a non-negative safe integer");
    }
    const ownerDocument = viewport.element.ownerDocument;
    this.element = ownerDocument.createElement("div");
    this.element.className = "aster-editor-read-only-message";
    this.element.hidden = true;
    this.element.textContent = message;
    this.element.setAttribute("role", "status");
    this.element.setAttribute("aria-live", "polite");
    viewport.element.append(this.element);
    this.defer(() => {
      if (this.hideTimer !== undefined) clearTimeout(this.hideTimer);
      this.element.remove();
    });
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || !isMutationKey(event)) return;
      stopEvent(event);
      this.show();
    }));
    this.own(addDisposableListener(input, "beforeinput", event => {
      if (event.defaultPrevented || !isMutationInput(event)) return;
      stopEvent(event);
      this.show();
    }));
    this.own(addDisposableListener(input, "paste", event => {
      stopEvent(event);
      this.show();
    }));
    this.own(addDisposableListener(input, "cut", event => {
      stopEvent(event);
      this.show();
    }));
  }

  show(): void {
    this.element.hidden = false;
    this.element.classList.add("visible");
    if (this.hideTimer !== undefined) clearTimeout(this.hideTimer);
    if (this.durationMs === 0) {
      this.hide();
      return;
    }
    this.hideTimer = setTimeout(() => {
      this.hideTimer = undefined;
      this.hide();
    }, this.durationMs);
  }

  hide(): void {
    if (this.hideTimer !== undefined) {
      clearTimeout(this.hideTimer);
      this.hideTimer = undefined;
    }
    this.element.hidden = true;
    this.element.classList.remove("visible");
  }
}

function isMutationKey(event: KeyboardEvent): boolean {
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return (event.ctrlKey || event.metaKey) && !event.shiftKey && (event.key.toLowerCase() === "x" || event.key.toLowerCase() === "v");
  }
  return event.key.length === 1 || event.key === "Backspace" || event.key === "Delete" || event.key === "Enter";
}

function isMutationInput(event: InputEvent): boolean {
  return event.inputType.startsWith("insert") || event.inputType.startsWith("delete") || event.inputType === "historyUndo" || event.inputType === "historyRedo";
}
