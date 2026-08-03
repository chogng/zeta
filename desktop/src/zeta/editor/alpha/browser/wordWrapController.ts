import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { AlphaEditorLineWrapping } from "./visualLineProjection.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

/** Owns the transient Alt+Z word-wrap toggle for one Alpha viewport. */
export class AlphaWordWrapController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
  ) {
    super();
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || event.key.toLowerCase() !== "z") return;
    stopEvent(event);
    this.viewport.setLineWrapping(this.viewport.lineWrapping === AlphaEditorLineWrapping.On
      ? AlphaEditorLineWrapping.Off
      : AlphaEditorLineWrapping.On);
  }
}
