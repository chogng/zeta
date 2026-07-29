import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface ScrollbarOptions {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly onScroll?: (position: { left: number; top: number }) => void;
}

/**
 * Themeable scroll container backed by the browser's native scrolling model.
 */
export class Scrollbar extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(options: ScrollbarOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-scrollbar";
    element.tabIndex = 0;
    if (options.ariaLabel) {
      element.setAttribute("role", "region");
      element.setAttribute("aria-label", options.ariaLabel);
    }
    if (options.onScroll) {
      this.own(addDisposableListener(element, "scroll", () =>
        options.onScroll?.({
          left: element.scrollLeft,
          top: element.scrollTop,
        }),
      ));
    }
  }

  setContent(content: Element): void { this.element.replaceChildren(content); }
  scrollTo(left: number, top: number): void { this.element.scrollTo({ left, top }); }
}
