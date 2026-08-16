import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { ObjectTreeModel, type ObjectTreeElement, type ObjectTreeModelOptions, type ObjectTreeNode } from "./objectTreeModel.js";
import { Tree, type TreeIndentGuides, type TreeTwistieState } from "./tree.js";

export interface ObjectTreeOptions<TNode> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly modelOptions: ObjectTreeModelOptions<TNode>;
  readonly renderElement: (element: TNode, node: ObjectTreeNode<TNode>) => HTMLElement;
  readonly renderTwistie?: (element: TNode, state: TreeTwistieState, container: HTMLSpanElement) => void;
}

export interface ObjectTreeActivateEvent<TNode> {
  readonly element: TNode;
  readonly node: ObjectTreeNode<TNode>;
  readonly browserEvent: MouseEvent | KeyboardEvent;
}

/** Model-driven accessible tree view for ordinary single-action rows. */
export class ObjectTree<TNode> extends DisposableOwner {
  readonly element: HTMLUListElement;
  readonly model: ObjectTreeModel<TNode>;
  private readonly tree: Tree<ObjectTreeNode<TNode>>;
  private readonly _onDidActivate = this.own(new Emitter<ObjectTreeActivateEvent<TNode>>());

  readonly onDidActivate: Event<ObjectTreeActivateEvent<TNode>> = this._onDidActivate.event;

  constructor(options: ObjectTreeOptions<TNode>) {
    super();
    this.model = this.own(new ObjectTreeModel(options.modelOptions));
    this.tree = this.own(new Tree({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      indent: options.indent,
      indentGuides: options.indentGuides,
      getId: (node) => node.id,
      getChildren: (node) => node.children.filter((child) => child.visible),
      isCollapsible: (node) => node.collapsible,
      isExpanded: (node) => !node.collapsed,
      renderElement: (node) => options.renderElement(node.element, node),
      renderTwistie: options.renderTwistie
        ? (node, state, container) => options.renderTwistie!(node.element, state, container)
        : undefined,
    }));
    this.element = this.tree.element;
    this.own(this.model.onDidChange(() => this.render()));
    this.own(this.tree.onDidActivate(({ element: node, browserEvent }) => {
      this._onDidActivate.fire({ element: node.element, node, browserEvent });
    }));
    this.render();
  }

  setChildren(children: readonly ObjectTreeElement<TNode>[]): void {
    this.model.setChildren(children);
  }

  setNodeChildren(parentId: string, children: readonly ObjectTreeElement<TNode>[]): void {
    this.model.setNodeChildren(parentId, children);
  }

  collapse(id: string): boolean {
    return this.model.collapse(id);
  }

  expand(id: string): boolean {
    return this.model.expand(id);
  }

  toggleCollapsed(id: string): boolean {
    return this.model.toggleCollapsed(id);
  }

  expandTo(id: string): boolean {
    return this.model.expandTo(id);
  }

  private render(): void {
    this.tree.items = this.model.visibleChildren;
  }
}
