import { Component } from "../common/component.js";

/** A scroll container that delegates scrolling to the browser's accessible native behavior. */
export class Scrollbar extends Component<HTMLDivElement> {
  constructor(onScroll?: (position: { left: number; top: number }) => void) {
    const element = document.createElement("div");
    element.className = "zeta-scrollbar";
    element.tabIndex = 0;
    super(element);
    if (onScroll) this.listen(element, "scroll", () => onScroll({ left: element.scrollLeft, top: element.scrollTop }));
  }

  setContent(content: Element): void { this.element.replaceChildren(content); }
  scrollTo(left: number, top: number): void { this.element.scrollTo({ left, top }); }
}
