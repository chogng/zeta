import { Component } from "../common/component.js";

/** A CSS grid container whose column and row templates are explicit. */
export class Grid extends Component<HTMLDivElement> {
  constructor(columns: string, rows = "auto") {
    const element = document.createElement("div");
    element.className = "zeta-grid";
    element.style.gridTemplateColumns = columns;
    element.style.gridTemplateRows = rows;
    super(element);
  }

  add(child: Element): void { this.element.append(child); }
}
