import { Component } from "../common/component.js";

/** A generic list renderer that leaves row presentation to its caller. */
export class List<T> extends Component<HTMLUListElement> {
  constructor(items: readonly T[], render: (item: T, index: number) => Element) {
    const element = document.createElement("ul");
    element.className = "zeta-list";
    super(element);
    this.setItems(items, render);
  }

  setItems(items: readonly T[], render: (item: T, index: number) => Element): void {
    this.element.replaceChildren(...items.map((item, index) => {
      const row = document.createElement("li");
      row.append(render(item, index));
      return row;
    }));
  }
}
