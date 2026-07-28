import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface TreeItem { id: string; label: string; children?: readonly TreeItem[]; }

/** A simple accessible tree renderer with selectable nodes. */
export class Tree extends DisposableOwner {
  readonly element: HTMLUListElement;

  constructor(items: readonly TreeItem[], onSelect?: (item: TreeItem) => void) {
    super();
    const element = document.createElement("ul");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-tree";
    element.setAttribute("role", "tree");
    this.render(items, element, onSelect);
  }

  private render(items: readonly TreeItem[], parent: HTMLElement, onSelect?: (item: TreeItem) => void): void {
    for (const item of items) {
      const node = document.createElement("li");
      node.textContent = item.label;
      node.tabIndex = 0;
      node.setAttribute("role", "treeitem");
      this.own(addDisposableListener(node, "click", () => onSelect?.(item)));
      parent.append(node);
      if (item.children?.length) {
        const children = document.createElement("ul");
        children.setAttribute("role", "group");
        node.append(children);
        this.render(item.children, children, onSelect);
      }
    }
  }
}
