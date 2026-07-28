import { DisposableOwner } from "../../../common/lifecycle.js";

/** A generic list renderer that leaves row presentation to its caller. */
export class List<T> extends DisposableOwner {
  readonly element: HTMLUListElement;

  constructor(items: readonly T[], render: (item: T, index: number) => Element) {
    super();
    const element = document.createElement("ul");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-list";
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
