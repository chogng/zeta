import { DisposableOwner } from "../../../common/lifecycle.js";

/** A CSS grid container whose column and row templates are explicit. */
export class Grid extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(columns: string, rows = "auto") {
    super();
    const element = document.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-grid";
    element.style.gridTemplateColumns = columns;
    element.style.gridTemplateRows = rows;
  }

  add(child: Element): void { this.element.append(child); }
}
