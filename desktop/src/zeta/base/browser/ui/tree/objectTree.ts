import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { AbstractTree } from "./abstractTree.js";
import { ObjectTreeModel, type ObjectTreeElement, type ObjectTreeModelOptions, type ObjectTreeNode } from "./objectTreeModel.js";
import type { TreeIndentGuides, TreePointerTarget, TreeTwistieState } from "./tree.js";

export interface ObjectTreeOptions<TNode> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly expandOnlyOnTwistieClick?: boolean | ((element: TNode) => boolean);
  readonly modelOptions: ObjectTreeModelOptions<TNode>;
  readonly onWillRender?: () => void;
  readonly renderElement: (element: TNode, node: ObjectTreeNode<TNode>) => HTMLElement;
  readonly renderTwistie?: (element: TNode, state: TreeTwistieState, container: HTMLSpanElement) => void;
}

export interface ObjectTreeActivateEvent<TNode> {
  readonly element: TNode;
  readonly node: ObjectTreeNode<TNode>;
  readonly browserEvent: MouseEvent | KeyboardEvent;
}

export interface ObjectTreePointerEvent<TNode> {
  readonly element: TNode;
  readonly node: ObjectTreeNode<TNode>;
  readonly target: TreePointerTarget;
  readonly browserEvent: MouseEvent;
}

export interface ObjectTreeAcceptEvent<TNode> {
  readonly element: TNode;
  readonly node: ObjectTreeNode<TNode>;
  readonly browserEvent: KeyboardEvent;
}

export interface ObjectTreeFocusChangeEvent<TNode> {
  readonly element: TNode | undefined;
  readonly node: ObjectTreeNode<TNode> | undefined;
  readonly browserEvent: UIEvent | undefined;
}

export interface ObjectTreeSelectionChangeEvent<TNode> {
  readonly elements: readonly TNode[];
  readonly nodes: readonly ObjectTreeNode<TNode>[];
  readonly browserEvent: UIEvent | undefined;
}

export interface ObjectTreeCollapseStateChangeEvent<TNode> {
  readonly element: TNode;
  readonly node: ObjectTreeNode<TNode>;
  readonly collapsed: boolean;
  readonly browserEvent: MouseEvent | KeyboardEvent | undefined;
}

/** Model-driven accessible tree view for ordinary single-action rows. */
export class ObjectTree<TNode> extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly model: ObjectTreeModel<TNode>;
  private readonly tree: AbstractTree<TNode, ObjectTreeNode<TNode>>;
  private readonly _onPointer = this.own(new Emitter<ObjectTreePointerEvent<TNode>>());
  private readonly _onDidDoubleClick = this.own(new Emitter<ObjectTreePointerEvent<TNode>>());
  private readonly _onDidAccept = this.own(new Emitter<ObjectTreeAcceptEvent<TNode>>());
  private readonly _onDidChangeFocus = this.own(new Emitter<ObjectTreeFocusChangeEvent<TNode>>());
  private readonly _onDidChangeSelection = this.own(new Emitter<ObjectTreeSelectionChangeEvent<TNode>>());
  private readonly _onDidChangeCollapseState = this.own(new Emitter<ObjectTreeCollapseStateChangeEvent<TNode>>());
  private readonly _onDidActivate = this.own(new Emitter<ObjectTreeActivateEvent<TNode>>());
  private readonly onWillRender: (() => void) | undefined;
  private collapseBrowserEvent: { readonly id: string; readonly event: MouseEvent | KeyboardEvent } | undefined;

  readonly onPointer: Event<ObjectTreePointerEvent<TNode>> = this._onPointer.event;
  readonly onDidDoubleClick: Event<ObjectTreePointerEvent<TNode>> = this._onDidDoubleClick.event;
  readonly onDidAccept: Event<ObjectTreeAcceptEvent<TNode>> = this._onDidAccept.event;
  readonly onDidChangeFocus: Event<ObjectTreeFocusChangeEvent<TNode>> = this._onDidChangeFocus.event;
  readonly onDidChangeSelection: Event<ObjectTreeSelectionChangeEvent<TNode>> = this._onDidChangeSelection.event;
  readonly onDidChangeCollapseState: Event<ObjectTreeCollapseStateChangeEvent<TNode>> = this._onDidChangeCollapseState.event;
  /** @deprecated Prefer the semantic pointer, accept, and collapse events. */
  readonly onDidActivate: Event<ObjectTreeActivateEvent<TNode>> = this._onDidActivate.event;

  constructor(options: ObjectTreeOptions<TNode>) {
    super();
    const expandOnlyOnTwistieClick = options.expandOnlyOnTwistieClick;
    this.onWillRender = options.onWillRender;
    this.model = this.own(new ObjectTreeModel(options.modelOptions));
    this.tree = this.own(new AbstractTree({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      indent: options.indent,
      indentGuides: options.indentGuides,
      expandOnlyOnTwistieClick: typeof expandOnlyOnTwistieClick === "function" ? (node) => expandOnlyOnTwistieClick(node.element) : expandOnlyOnTwistieClick,
      renderElement: (node) => options.renderElement(node.element, node),
      renderTwistie: options.renderTwistie
        ? (node, state, container) => options.renderTwistie!(node.element, state, container)
        : undefined,
    }));
    this.element = this.tree.element;
    this.own(this.model.onDidChange(() => this.render()));
    this.own(this.model.onDidChangeCollapseState(({ node, collapsed }) => {
      this._onDidChangeCollapseState.fire({ element: node.element, node, collapsed, browserEvent: this.collapseBrowserEvent?.id === node.id ? this.collapseBrowserEvent.event : undefined });
    }));
    this.own(this.tree.onPointer(({ element: node, target, browserEvent }) => {
      this._onPointer.fire({ element: node.element, node, target, browserEvent });
    }));
    this.own(this.tree.onDidDoubleClick(({ element: node, target, browserEvent }) => {
      this._onDidDoubleClick.fire({ element: node.element, node, target, browserEvent });
    }));
    this.own(this.tree.onDidAccept(({ element: node, browserEvent }) => {
      this._onDidAccept.fire({ element: node.element, node, browserEvent });
    }));
    this.own(this.tree.onDidChangeFocus(({ element: node, browserEvent }) => {
      this._onDidChangeFocus.fire({ element: node?.element, node, browserEvent });
    }));
    this.own(this.tree.onDidChangeSelection(({ elements: nodes, browserEvent }) => {
      this._onDidChangeSelection.fire({ elements: nodes.map((node) => node.element), nodes, browserEvent });
    }));
    this.own(this.tree.onDidRequestCollapseChange(({ element: node, expanded, browserEvent }) => {
      this.collapseBrowserEvent = { id: node.id, event: browserEvent };
      try {
        if (expanded) this.model.expand(node.id);
        else this.model.collapse(node.id);
      } finally {
        this.collapseBrowserEvent = undefined;
      }
    }));
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

  collapseRecursive(id: string): boolean {
    return this.model.collapseRecursive(id);
  }

  expandRecursive(id: string): boolean {
    return this.model.expandRecursive(id);
  }

  expandTo(id: string): boolean {
    return this.model.expandTo(id);
  }

  get focus(): TNode | undefined {
    return this.tree.focus?.element;
  }

  get selection(): readonly TNode[] {
    return this.tree.selection.map((node) => node.element);
  }

  setFocus(id: string, browserEvent?: UIEvent): void {
    this.tree.setFocus(id, browserEvent);
  }

  setSelection(ids: readonly string[], browserEvent?: UIEvent): void {
    this.tree.setSelection(ids, browserEvent);
  }

  private render(): void {
    this.onWillRender?.();
    this.tree.items = this.model.visibleNodes;
  }
}
