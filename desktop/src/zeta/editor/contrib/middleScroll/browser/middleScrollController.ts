import "./media/middleScroll.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Implements editor-local middle-button panning without entering pointer selection mode. */
export class MiddleScrollController extends DisposableOwner {
  private active: { readonly pointerId: number; readonly x: number; readonly y: number; readonly left: number; readonly top: number } | undefined;

  constructor(private readonly viewport: EditorViewport) {
    super();
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointerdown", event => {
      if (event.button !== 1) return;
      event.preventDefault();
      viewport.element.setPointerCapture?.(event.pointerId);
      const scroll = viewport.viewportLayout.scrollPosition;
      this.active = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, left: scroll.left, top: scroll.top };
      viewport.element.classList.add("middle-scrolling");
    }));
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointermove", event => {
      if (!this.active || this.active.pointerId !== event.pointerId) return;
      event.preventDefault();
      viewport.scrollTo({ left: this.active.left - event.clientX + this.active.x, top: this.active.top - event.clientY + this.active.y });
    }));
    const end = (event: PointerEvent): void => { if (this.active?.pointerId !== event.pointerId) return; this.active = undefined; viewport.element.classList.remove("middle-scrolling"); viewport.element.releasePointerCapture?.(event.pointerId); };
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointerup", end));
    this.own(addDisposableListener<PointerEvent>(viewport.element, "pointercancel", end));
    this.defer(() => { this.active = undefined; viewport.element.classList.remove("middle-scrolling"); });
  }
}
