import { ObjectTreeModel, type ObjectTreeElement, type ObjectTreeNode } from "../../../../base/browser/ui/tree/objectTreeModel.js";
import { TreeVisibility } from "../../../../base/browser/ui/tree/tree.js";

export interface SettingsTreeItem<T> {
  readonly kind: "item";
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly keywords?: readonly string[];
  readonly value: T;
}

export interface SettingsTreeGroup {
  readonly kind: "group";
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly keywords?: readonly string[];
}

export type SettingsTreeElement<T> = SettingsTreeGroup | SettingsTreeItem<T>;
export type SettingsTreeNode<T> = ObjectTreeElement<SettingsTreeElement<T>>;

/** Settings group/item model with query filtering over canonical item metadata. */
export class SettingsTreeModel<T> extends ObjectTreeModel<SettingsTreeElement<T>> {
  private queryTerms: readonly string[] = [];
  private _query = "";

  constructor() {
    super({ identityProvider: { getId: (node) => node.id } });
    this.setFilter({ filter: (node) => this.filterNode(node) });
  }

  get query(): string {
    return this._query;
  }

  get visibleItems(): readonly SettingsTreeItem<T>[] {
    return this.visibleNodes
      .map((node) => node.element)
      .filter((node): node is SettingsTreeItem<T> => node.kind === "item");
  }

  setChildren(children: readonly SettingsTreeNode<T>[]): void {
    validateSettingsNodes(children);
    super.setChildren(asNonCollapsibleSettingsTree(children));
  }

  setNodeChildren(parentId: string, children: readonly SettingsTreeNode<T>[]): void {
    validateSettingsNodes(children);
    super.setNodeChildren(parentId, asNonCollapsibleSettingsTree(children));
  }

  setQuery(query: string): void {
    const normalized = normalizeSearchText(query);
    if (normalized === this._query) return;
    this._query = normalized;
    this.queryTerms = normalized ? normalized.split(/\s+/u) : [];
    this.refilter();
  }

  getGroup(id: string): SettingsTreeGroup | undefined {
    const node = this.getElement(id);
    return node?.kind === "group" ? node : undefined;
  }

  getItem(id: string): SettingsTreeItem<T> | undefined {
    const node = this.getElement(id);
    return node?.kind === "item" ? node : undefined;
  }

  countVisibleItems(groupId?: string): number {
    const roots = groupId === undefined
      ? this.visibleChildren
      : this.getNode(groupId)?.children.filter((node) => node.visible) ?? [];
    return roots.reduce((count, node) => count + countVisibleItems(node), 0);
  }

  private filterNode(node: SettingsTreeElement<T>): boolean | TreeVisibility {
    if (this.queryTerms.length === 0) return TreeVisibility.Visible;
    if (node.kind === "group") return TreeVisibility.Recurse;
    const text = normalizeSearchText([node.title, node.description, ...(node.keywords ?? [])].join(" "));
    return this.queryTerms.every((term) => text.includes(term));
  }
}

function countVisibleItems<T>(node: ObjectTreeNode<SettingsTreeElement<T>>): number {
  if (!node.visible) return 0;
  if (node.element.kind === "item") return 1;
  return node.children.reduce((count, child) => count + countVisibleItems(child), 0);
}

function validateSettingsNodes<T>(nodes: readonly SettingsTreeNode<T>[]): void {
  const visit = (node: SettingsTreeNode<T>): void => {
    const element = node.element;
    if (!element.title.trim()) throw new TypeError(`Settings tree ${element.kind} '${element.id}' must have a title`);
    if (element.kind === "item" && (node.children?.length ?? 0) > 0) {
      throw new TypeError(`Settings tree item '${element.id}' must not have children`);
    }
    for (const child of node.children ?? []) visit(child);
  };
  for (const node of nodes) visit(node);
}

function asNonCollapsibleSettingsTree<T>(nodes: readonly SettingsTreeNode<T>[]): readonly SettingsTreeNode<T>[] {
  return nodes.map((node) => ({
    element: node.element,
    collapsible: false,
    children: asNonCollapsibleSettingsTree(node.children ?? []),
  }));
}

function normalizeSearchText(value: string): string {
  return value.trim().toLocaleLowerCase();
}
