import { DisposableOwner } from "../../../common/lifecycle.js";
import { Sash } from "../sash/sash.js";

export type SplitViewOrientation = "horizontal" | "vertical";

/** A flex layout with panes separated by draggable sashes. */
export class SplitView extends DisposableOwner {
  readonly element: HTMLDivElement;
  #panes: HTMLElement[] = [];

  constructor(
    readonly orientation: SplitViewOrientation,
    ownerDocument: Document = document,
  ) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-split-view zeta-split-view-${orientation}`;
    element.style.flexDirection = orientation === "horizontal" ? "row" : "column";
  }

  addPane(content: Element, basis = "1fr"): void {
    const ownerDocument = this.element.ownerDocument;
    const pane = ownerDocument.createElement("div");
    pane.className = "zeta-split-view-pane";
    pane.style.flex = flexForPaneSize(basis);
    pane.append(content);
    const previous = this.#panes.at(-1);
    if (previous) {
      const sash = this.own(
        new Sash(
          this.orientation === "horizontal" ? "vertical" : "horizontal",
          ownerDocument,
        ),
      );
      this.own(
        sash.onDidDrag((delta) =>
          this.resizeAdjacentPanes(previous, pane, delta),
        ),
      );
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
