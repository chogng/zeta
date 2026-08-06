import "./media/toggleTabFocusMode.css";
import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";

/** Controls whether Tab is routed to editor text insertion or browser focus traversal. */
export class AlphaToggleTabFocusModeController extends DisposableOwner {
  private enabled = false;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: AlphaEditorViewport) {
    super();
    this.own(addDisposableListener(input, "keydown", event => this.handleToggle(event), true));
    this.own(addDisposableListener(input, "keydown", event => {
      if (this.enabled && !event.defaultPrevented && !event.isComposing && event.key === "Tab" && !event.ctrlKey && !event.altKey && !event.metaKey) {
        event.stopImmediatePropagation();
        this.viewport.element.classList.add("tab-focus-mode-active");
      }
    }, true));
    this.own(addDisposableListener(input, "blur", () => this.viewport.element.classList.remove("tab-focus-mode-active")));
    this.updateState();
  }

  get isEnabled(): boolean { return this.enabled; }

  setEnabled(enabled: boolean): void { this.enabled = Boolean(enabled); this.updateState(); }

  private handleToggle(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.key.toLowerCase() !== "m" || !event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
    stopEvent(event, { immediate: true });
    this.setEnabled(!this.enabled);
    this.viewport.announceAccessibilityStatus(this.enabled ? "Tab moves focus out of the editor" : "Tab inserts indentation");
  }

  private updateState(): void {
    this.viewport.element.classList.toggle("tab-focus-mode", this.enabled);
    this.viewport.element.dataset.tabFocusMode = String(this.enabled);
  }
}
