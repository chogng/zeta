import { Component } from "../common/component.js";
import { Sash } from "../sash/sash.js";

export type SplitViewOrientation = "horizontal" | "vertical";

/** A flex layout with panes separated by draggable sashes. */
export class SplitView extends Component<HTMLDivElement> {
  #panes: HTMLElement[] = [];

  constructor(readonly orientation: SplitViewOrientation) {
    const element = document.createElement("div");
    element.className = `zeta-split-view zeta-split-view-${orientation}`;
    element.style.flexDirection = orientation === "horizontal" ? "row" : "column";
    super(element);
  }

  addPane(content: Element, basis = "1fr"): void {
    const pane = document.createElement("div");
    pane.className = "zeta-split-view-pane";
    pane.style.flex = flexForPaneSize(basis);
    pane.append(content);
    const previous = this.#panes.at(-1);
    if (previous) {
      const sash = new Sash(this.orientation === "horizontal" ? "vertical" : "horizontal");
      sash.onDidDrag((delta) => this.resizeAdjacentPanes(previous, pane, delta));
      this.element.append(sash.element);
    }
    this.#panes.push(pane);
    this.element.append(pane);
  }

  private resizeAdjacentPanes(previous: HTMLElement, next: HTMLElement, delta: number): void {
    const axis = this.orientation === "horizontal" ? "width" : "height";
    const previousSize = previous.getBoundingClientRect()[axis] + delta;
    const nextSize = next.getBoundingClientRect()[axis] - delta;
    if (previousSize < 40 || nextSize < 40) return;
    previous.style.flex = `0 0 ${previousSize}px`;
    next.style.flex = `0 0 ${nextSize}px`;
  }
}

function flexForPaneSize(size: string): string {
  const fractional = /^(?<factor>\d*\.?\d+)fr$/.exec(size);
  if (fractional?.groups?.factor) return `${fractional.groups.factor} 1 0px`;
  return `0 0 ${size}`;
}
