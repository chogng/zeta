import { Component } from "../common/component.js";

/** An anchored, transient container for menus, hovers, and other overlays. */
export class ContextView extends Component<HTMLDivElement> {
  constructor() {
    const element = document.createElement("div");
    element.className = "zeta-context-view";
    element.hidden = true;
    super(element);
    document.body.append(element);
  }

  show(anchor: Element, content: Element): void {
    const bounds = anchor.getBoundingClientRect();
    this.element.replaceChildren(content);
    this.element.style.left = `${bounds.left}px`;
    this.element.style.top = `${bounds.bottom}px`;
    this.element.hidden = false;
  }

  hide(): void { this.element.hidden = true; this.element.replaceChildren(); }
}
