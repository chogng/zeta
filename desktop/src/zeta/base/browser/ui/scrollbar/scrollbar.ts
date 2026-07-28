import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

/** A scroll container that delegates scrolling to the browser's accessible native behavior. */
export class Scrollbar extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(onScroll?: (position: { left: number; top: number }) => void) {
    super();
    const element = document.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-scrollbar";
    element.tabIndex = 0;
    if (onScroll) {
      this.own(addDisposableListener(element, "scroll", () =>
        onScroll({ left: element.scrollLeft, top: element.scrollTop }),
      ));
    }
  }

  setContent(content: Element): void { this.element.replaceChildren(content); }
  scrollTo(left: number, top: number): void { this.element.scrollTo({ left, top }); }
}
