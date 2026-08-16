import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { TreeVisibility, type TreeFilter, type TreeFilterResult, type TreeSorter } from "./tree.js";

export type ObjectTreeDefaultCollapseState = "collapsed" | "expanded";
export type ObjectTreeModelChangeKind = "structure" | "collapse" | "filter" | "sort" | "rerender";

/** One structural entry supplied to `ObjectTreeModel`. */
export interface ObjectTreeElement<T> {
  readonly element: T;
  readonly children?: readonly ObjectTreeElement<T>[];
  readonly collapsible?: boolean;
  readonly collapsed?: boolean;
}

export interface ObjectTreeIdentityProvider<T> {
  getId(element: T): string;
}

/** Canonical structural node derived from one domain object. */
export interface ObjectTreeNode<TNode> {
  readonly id: string;
  readonly element: TNode;
  readonly parent: ObjectTreeNode<TNode> | undefined;
  readonly children: readonly ObjectTreeNode<TNode>[];
  readonly depth: number;
  readonly collapsible: boolean;
  readonly collapsed: boolean;
  readonly visible: boolean;
  readonly visibleChildIndex: number;
  readonly visibleChildrenCount: number;
}

export interface ObjectTreeModelOptions<TNode> {
  readonly defaultCollapseState?: ObjectTreeDefaultCollapseState;
  readonly filter?: TreeFilter<TNode>;
  readonly identityProvider: ObjectTreeIdentityProvider<TNode>;
  readonly sorter?: TreeSorter<TNode>;
}

export interface ObjectTreeModelChangeEvent<TNode> {
  readonly kind: ObjectTreeModelChangeKind;
  readonly node: ObjectTreeNode<TNode> | undefined;
}

interface MutableObjectTreeNode<TNode> {
  readonly id: string;
  readonly element: TNode;
  readonly parent: MutableObjectTreeNode<TNode> | undefined;
  children: MutableObjectTreeNode<TNode>[];
  readonly depth: number;
  readonly declaredCollapsible: boolean | undefined;
  collapsible: boolean;
  collapsed: boolean;
  visible: boolean;
  visibleChildIndex: number;
  visibleChildrenCount: number;
}

/**
 * Owns an object hierarchy independently from its DOM renderer.
 *
 * Stable IDs preserve collapse state across structural replacement. The model
 * owns validation, parent/depth metadata, local child replacement, filtering,
 * sorting, and the flattened visible-node projection consumed by tree views.
 */
export class ObjectTreeModel<TNode> extends DisposableOwner {
  private readonly _onDidChange = this.own(new Emitter<ObjectTreeModelChangeEvent<TNode>>());
  private readonly nodesById = new Map<string, MutableObjectTreeNode<TNode>>();
  private roots: MutableObjectTreeNode<TNode>[] = [];
  private _visibleNodes: readonly ObjectTreeNode<TNode>[] = [];
  private filter: TreeFilter<TNode> | undefined;
  private sorter: TreeSorter<TNode> | undefined;
  private readonly defaultCollapseState: ObjectTreeDefaultCollapseState;
  private readonly identityProvider: ObjectTreeIdentityProvider<TNode>;

  readonly onDidChange: Event<ObjectTreeModelChangeEvent<TNode>> = this._onDidChange.event;

  constructor(options: ObjectTreeModelOptions<TNode>) {
    super();
    this.defaultCollapseState = options.defaultCollapseState ?? "expanded";
    this.filter = options.filter;
    this.identityProvider = options.identityProvider;
    this.sorter = options.sorter;
  }

  get children(): readonly TNode[] {
    return this.roots.map((node) => node.element);
  }

  get rootNodes(): readonly ObjectTreeNode<TNode>[] {
    return this.roots;
  }

  get visibleChildren(): readonly ObjectTreeNode<TNode>[] {
    return this.roots.filter((node) => node.visible);
  }

  get visibleNodes(): readonly ObjectTreeNode<TNode>[] {
    return this._visibleNodes;
  }

  get size(): number {
    return this.nodesById.size;
  }

  has(id: string): boolean {
    return this.nodesById.has(id);
  }

  getNode(id: string): ObjectTreeNode<TNode> | undefined {
    return this.nodesById.get(id);
  }

  getElement(id: string): TNode | undefined {
    return this.nodesById.get(id)?.element;
  }

  getParent(id: string): ObjectTreeNode<TNode> | undefined {
    return this.nodesById.get(id)?.parent;
  }

  setChildren(children: readonly ObjectTreeElement<TNode>[]): void {
    const previousNodes = new Map(this.nodesById);
    const seen = new Set<string>();
    const roots = this.buildNodes(children, undefined, previousNodes, seen);
    this.roots = roots;
    this.rebuildIndex();
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "structure", node: undefined });
  }

  setNodeChildren(parentId: string, children: readonly ObjectTreeElement<TNode>[]): void {
    const parent = this.requireMutableNode(parentId);
    const previousNodes = new Map(this.nodesById);
    const seen = new Set(this.nodesById.keys());
    for (const child of parent.children) this.removeNodeIds(child, seen);
    const nextChildren = this.buildNodes(children, parent, previousNodes, seen);
    parent.children = nextChildren;
    parent.collapsible = parent.declaredCollapsible ?? nextChildren.length > 0;
    if (!parent.collapsible) parent.collapsed = false;
    this.rebuildIndex();
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "structure", node: parent });
  }

  collapse(id: string): boolean {
    return this.updateCollapsed(id, "collapsed");
  }

  expand(id: string): boolean {
    return this.updateCollapsed(id, "expanded");
  }

  toggleCollapsed(id: string): boolean {
    const node = this.requireMutableNode(id);
    return this.updateCollapsed(id, node.collapsed ? "expanded" : "collapsed");
  }

  collapseRecursive(id: string): boolean {
    return this.updateCollapsedRecursive(id, "collapsed");
  }

  expandRecursive(id: string): boolean {
    return this.updateCollapsedRecursive(id, "expanded");
  }

  expandTo(id: string): boolean {
    let node = this.requireMutableNode(id).parent;
    let changed = false;
    while (node) {
      if (node.collapsible && node.collapsed) {
        node.collapsed = false;
        changed = true;
      }
      node = node.parent;
    }
    if (!changed) return false;
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "collapse", node: this.nodesById.get(id) });
    return true;
  }

  setFilter(filter: TreeFilter<TNode> | undefined): void {
    if (filter === this.filter) return;
    this.filter = filter;
    this.refilter();
  }

  refilter(): void {
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "filter", node: undefined });
  }

  setSorter(sorter: TreeSorter<TNode> | undefined): void {
    if (sorter === this.sorter) return;
    this.sorter = sorter;
    this.resort();
  }

  resort(): void {
    if (this.sorter) {
      this.sortNodes(this.roots);
      this.recomputeVisibleNodes();
    }
    this._onDidChange.fire({ kind: "sort", node: undefined });
  }

  rerender(id?: string): void {
    this._onDidChange.fire({ kind: "rerender", node: id === undefined ? undefined : this.requireMutableNode(id) });
  }

  private buildNodes(elements: readonly ObjectTreeElement<TNode>[], parent: MutableObjectTreeNode<TNode> | undefined, previousNodes: ReadonlyMap<string, MutableObjectTreeNode<TNode>>, seen: Set<string>): MutableObjectTreeNode<TNode>[] {
    const ordered = [...elements];
    if (this.sorter) ordered.sort((left, right) => this.sorter!.compare(left.element, right.element));
    return ordered.map((treeElement) => {
      const id = this.identityProvider.getId(treeElement.element);
      validateNodeId(id, seen);
      const previous = previousNodes.get(id);
      const node: MutableObjectTreeNode<TNode> = {
        id,
        element: treeElement.element,
        parent,
        children: [],
        depth: parent ? parent.depth + 1 : 1,
        declaredCollapsible: treeElement.collapsible,
        collapsible: false,
        collapsed: false,
        visible: true,
        visibleChildIndex: 0,
        visibleChildrenCount: 0,
      };
      node.children = this.buildNodes(treeElement.children ?? [], node, previousNodes, seen);
      node.collapsible = treeElement.collapsible ?? node.children.length > 0;
      node.collapsed = node.collapsible
        ? previous?.collapsed ?? treeElement.collapsed ?? this.defaultCollapseState === "collapsed"
        : false;
      return node;
    });
  }

  private rebuildIndex(): void {
    this.nodesById.clear();
    const visit = (node: MutableObjectTreeNode<TNode>): void => {
      this.nodesById.set(node.id, node);
      for (const child of node.children) visit(child);
    };
    for (const root of this.roots) visit(root);
  }

  private removeNodeIds(node: MutableObjectTreeNode<TNode>, ids: Set<string>): void {
    ids.delete(node.id);
    for (const child of node.children) this.removeNodeIds(child, ids);
  }

  private requireMutableNode(id: string): MutableObjectTreeNode<TNode> {
    const node = this.nodesById.get(id);
    if (!node) throw new RangeError(`Unknown tree node ID: ${id}`);
    return node;
  }

  private updateCollapsed(id: string, state: ObjectTreeDefaultCollapseState): boolean {
    const node = this.requireMutableNode(id);
    if (!node.collapsible) return false;
    const collapsed = state === "collapsed";
    if (node.collapsed === collapsed) return false;
    node.collapsed = collapsed;
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "collapse", node });
    return true;
  }

  private updateCollapsedRecursive(id: string, state: ObjectTreeDefaultCollapseState): boolean {
    const root = this.requireMutableNode(id);
    const collapsed = state === "collapsed";
    let changed = false;
    const visit = (node: MutableObjectTreeNode<TNode>): void => {
      if (node.collapsible && node.collapsed !== collapsed) {
        node.collapsed = collapsed;
        changed = true;
      }
      for (const child of node.children) visit(child);
    };
    visit(root);
    if (!changed) return false;
    this.recomputeVisibleNodes();
    this._onDidChange.fire({ kind: "collapse", node: root });
    return true;
  }

  private sortNodes(nodes: MutableObjectTreeNode<TNode>[]): void {
    nodes.sort((left, right) => this.sorter!.compare(left.element, right.element));
    for (const node of nodes) this.sortNodes(node.children);
  }

  private recomputeVisibleNodes(): void {
    for (const root of this.roots) this.updateFilterVisibility(root, TreeVisibility.Visible);
    this.updateVisibleChildMetadata(this.roots);
    const visibleNodes: ObjectTreeNode<TNode>[] = [];
    const append = (node: MutableObjectTreeNode<TNode>): void => {
      if (!node.visible) return;
      visibleNodes.push(node);
      if (node.collapsed) return;
      for (const child of node.children) append(child);
    };
    for (const root of this.roots) append(root);
    this._visibleNodes = visibleNodes;
  }

  private updateFilterVisibility(node: MutableObjectTreeNode<TNode>, parentVisibility: TreeVisibility): boolean {
    const visibility = normalizeFilterResult(this.filter?.filter(node.element, parentVisibility) ?? TreeVisibility.Visible);
    if (visibility === TreeVisibility.Hidden) {
      this.hideSubtree(node);
      return false;
    }
    let visibleDescendants = false;
    for (const child of node.children) {
      if (this.updateFilterVisibility(child, visibility)) visibleDescendants = true;
    }
    node.visible = visibility === TreeVisibility.Visible || visibleDescendants;
    return node.visible;
  }

  private hideSubtree(node: MutableObjectTreeNode<TNode>): void {
    node.visible = false;
    node.visibleChildIndex = -1;
    node.visibleChildrenCount = 0;
    for (const child of node.children) this.hideSubtree(child);
  }

  private updateVisibleChildMetadata(nodes: readonly MutableObjectTreeNode<TNode>[]): void {
    const visible = nodes.filter((node) => node.visible);
    for (const node of nodes) {
      node.visibleChildIndex = -1;
      node.visibleChildrenCount = visible.length;
    }
    for (let index = 0; index < visible.length; index += 1) {
      visible[index]!.visibleChildIndex = index;
    }
    for (const node of nodes) this.updateVisibleChildMetadata(node.children);
  }
}

function validateNodeId(id: string, seen: Set<string>): void {
  if (!id.trim()) throw new TypeError("Tree node IDs must not be empty");
  if (seen.has(id)) throw new Error(`Duplicate tree node ID: ${id}`);
  seen.add(id);
}

function normalizeFilterResult(result: TreeFilterResult): TreeVisibility {
  if (result === true) return TreeVisibility.Visible;
  if (result === false) return TreeVisibility.Hidden;
  return result;
}
