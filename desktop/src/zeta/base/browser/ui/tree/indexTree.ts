import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { AbstractTree } from "./abstractTree.js";
import { IndexTreeModel, type IndexTreeModelOptions, type IndexTreeNode } from "./indexTreeModel.js";
import type { IndexTreeLocation, TreeElement, TreeIndentGuides, TreePointerTarget, TreeTwistieState } from "./tree.js";

export interface IndexTreeOptions<T> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly expandOnlyOnTwistieClick?: boolean | ((element: T) => boolean);
  readonly modelOptions?: IndexTreeModelOptions<T>;
  readonly renderElement: (element: T, node: IndexTreeNode<T>) => HTMLElement;
  readonly renderTwistie?: (element: T, state: TreeTwistieState, container: HTMLSpanElement) => void;
}

export interface IndexTreePointerEvent<T> {
  readonly element: T;
  readonly node: IndexTreeNode<T>;
  readonly target: TreePointerTarget;
  readonly browserEvent: MouseEvent;
}

export interface IndexTreeAcceptEvent<T> {
  readonly element: T;
  readonly node: IndexTreeNode<T>;
  readonly browserEvent: KeyboardEvent;
}

export interface IndexTreeEvent<T> {
  readonly element: T | undefined;
  readonly node: IndexTreeNode<T> | undefined;
  readonly browserEvent: UIEvent | undefined;
}

export interface IndexTreeSelectionEvent<T> {
  readonly elements: readonly T[];
  readonly nodes: readonly IndexTreeNode<T>[];
  readonly browserEvent: UIEvent | undefined;
}

export interface IndexTreeCollapseStateChangeEvent<T> {
  readonly element: T;
  readonly node: IndexTreeNode<T>;
  readonly collapsed: boolean;
  readonly browserEvent: MouseEvent | KeyboardEvent | undefined;
}

/** Index-path tree widget backed directly by `IndexTreeModel.splice`. */
export class IndexTree<T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly model: IndexTreeModel<T>;
  private readonly tree: AbstractTree<T, IndexTreeNode<T>>;
  private readonly _onPointer = this.own(new Emitter<IndexTreePointerEvent<T>>());
  private readonly _onDidDoubleClick = this.own(new Emitter<IndexTreePointerEvent<T>>());
  private readonly _onDidAccept = this.own(new Emitter<IndexTreeAcceptEvent<T>>());
  private readonly _onDidChangeFocus = this.own(new Emitter<IndexTreeEvent<T>>());
  private readonly _onDidChangeSelection = this.own(new Emitter<IndexTreeSelectionEvent<T>>());
  private readonly _onDidChangeCollapseState = this.own(new Emitter<IndexTreeCollapseStateChangeEvent<T>>());
  private collapseBrowserEvent: { readonly id: string; readonly event: MouseEvent | KeyboardEvent } | undefined;

  readonly onPointer: Event<IndexTreePointerEvent<T>> = this._onPointer.event;
  readonly onDidDoubleClick: Event<IndexTreePointerEvent<T>> = this._onDidDoubleClick.event;
  readonly onDidAccept: Event<IndexTreeAcceptEvent<T>> = this._onDidAccept.event;
  readonly onDidChangeFocus: Event<IndexTreeEvent<T>> = this._onDidChangeFocus.event;
  readonly onDidChangeSelection: Event<IndexTreeSelectionEvent<T>> = this._onDidChangeSelection.event;
  readonly onDidChangeCollapseState: Event<IndexTreeCollapseStateChangeEvent<T>> = this._onDidChangeCollapseState.event;

  constructor(rootElement: T, options: IndexTreeOptions<T>) {
    super();
    const expandOnlyOnTwistieClick = options.expandOnlyOnTwistieClick;
    this.model = this.own(new IndexTreeModel(rootElement, options.modelOptions));
    this.tree = this.own(new AbstractTree({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      indent: options.indent,
      indentGuides: options.indentGuides,
      expandOnlyOnTwistieClick: typeof expandOnlyOnTwistieClick === "function" ? (node) => expandOnlyOnTwistieClick(node.element) : expandOnlyOnTwistieClick,
      renderElement: (node) => options.renderElement(node.element, node),
      renderTwistie: options.renderTwistie ? (node, state, container) => options.renderTwistie!(node.element, state, container) : undefined,
    }));
    this.element = this.tree.element;
    this.own(this.model.onDidChange(() => this.render()));
    this.own(this.model.onDidChangeCollapseState(({ node, collapsed }) => {
      this._onDidChangeCollapseState.fire({ element: node.element, node, collapsed, browserEvent: this.collapseBrowserEvent?.id === node.id ? this.collapseBrowserEvent.event : undefined });
    }));
    this.own(this.tree.onPointer(({ element: node, target, browserEvent }) => this._onPointer.fire({ element: node.element, node, target, browserEvent })));
    this.own(this.tree.onDidDoubleClick(({ element: node, target, browserEvent }) => this._onDidDoubleClick.fire({ element: node.element, node, target, browserEvent })));
    this.own(this.tree.onDidAccept(({ element: node, browserEvent }) => this._onDidAccept.fire({ element: node.element, node, browserEvent })));
    this.own(this.tree.onDidChangeFocus(({ element: node, browserEvent }) => this._onDidChangeFocus.fire({ element: node?.element, node, browserEvent })));
    this.own(this.tree.onDidChangeSelection(({ elements: nodes, browserEvent }) => this._onDidChangeSelection.fire({ elements: nodes.map((node) => node.element), nodes, browserEvent })));
    this.own(this.tree.onDidRequestCollapseChange(({ element: node, expanded, browserEvent }) => {
      this.collapseBrowserEvent = { id: node.id, event: browserEvent };
      try {
        if (expanded) this.model.expand(node.location);
        else this.model.collapse(node.location);
      } finally {
        this.collapseBrowserEvent = undefined;
      }
    }));
    this.render();
  }

  splice(location: IndexTreeLocation, deleteCount: number, toInsert: readonly TreeElement<T>[] = []): void { this.model.splice(location, deleteCount, toInsert); }
  setChildren(children: readonly TreeElement<T>[]): void { this.model.setChildren(children); }
  collapse(location: IndexTreeLocation): boolean { return this.model.collapse(location); }
  expand(location: IndexTreeLocation): boolean { return this.model.expand(location); }
  toggleCollapsed(location: IndexTreeLocation): boolean { return this.model.toggleCollapsed(location); }
  expandTo(location: IndexTreeLocation): boolean { return this.model.expandTo(location); }
  get focus(): T | undefined { return this.tree.focus?.element; }
  get selection(): readonly T[] { return this.tree.selection.map((node) => node.element); }

  setFocus(location: IndexTreeLocation, browserEvent?: UIEvent): void {
    this.tree.setFocus(this.model.getNode(location).id, browserEvent);
  }

  setSelection(locations: readonly IndexTreeLocation[], browserEvent?: UIEvent): void {
    this.tree.setSelection(locations.map((location) => this.model.getNode(location).id), browserEvent);
  }

  private render(): void {
    this.tree.items = this.model.visibleNodes;
  }
}
