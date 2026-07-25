import { Component } from "../common/component.js";

export interface TreeItem { id: string; label: string; children?: readonly TreeItem[]; }

/** A simple accessible tree renderer with selectable nodes. */
export class Tree extends Component<HTMLUListElement> {
  constructor(items: readonly TreeItem[], onSelect?: (item: TreeItem) => void) {
    const element = document.createElement("ul");
    element.className = "zeta-tree";
    element.setAttribute("role", "tree");
    super(element);
    this.render(items, element, onSelect);
  }

  private render(items: readonly TreeItem[], parent: HTMLElement, onSelect?: (item: TreeItem) => void): void {
    for (const item of items) {
      const node = document.createElement("li");
      node.textContent = item.label;
      node.tabIndex = 0;
      node.setAttribute("role", "treeitem");
      this.listen(node, "click", () => onSelect?.(item));
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
