import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { ObjectTree, type ObjectTreeAcceptEvent, type ObjectTreeCollapseStateChangeEvent, type ObjectTreeFocusChangeEvent, type ObjectTreePointerEvent, type ObjectTreeSelectionChangeEvent } from "./objectTree.js";
import type { ObjectTreeElement, ObjectTreeIdentityProvider, ObjectTreeNode } from "./objectTreeModel.js";
import type { AsyncTreeDataSource, TreeFilter, TreeIndentGuides, TreeSorter, TreeTwistieState } from "./tree.js";

export interface AsyncTreeTwistieState extends TreeTwistieState {
  readonly loading: boolean;
}

export interface AsyncDataTreeOptions<T> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly expandOnlyOnTwistieClick?: boolean | ((element: T) => boolean);
  readonly identityProvider?: ObjectTreeIdentityProvider<T>;
  readonly sorter?: TreeSorter<T>;
  readonly filter?: TreeFilter<T>;
  readonly collapseByDefault?: (element: T) => boolean;
  readonly onWillRender?: () => void;
  readonly renderElement: (element: T, node: ObjectTreeNode<T>) => HTMLElement;
  readonly renderTwistie?: (element: T, state: AsyncTreeTwistieState, container: HTMLSpanElement) => void;
}

export interface AsyncDataTreeLoadStateEvent<T> {
  readonly element: T | undefined;
  readonly loading: boolean;
}

export interface AsyncDataTreeErrorEvent<T> {
  readonly element: T | undefined;
  readonly error: unknown;
}

interface AsyncNodeState<T> {
  readonly id: string;
  readonly element: T;
  readonly parentId: string | undefined;
  readonly hasChildren: boolean;
  readonly children: readonly string[] | undefined;
}

/** Lazy, race-safe data-source adapter over `ObjectTree`. */
export class AsyncDataTree<TInput, T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly tree: ObjectTree<T>;
  private readonly generatedIds = new Map<T, string>();
  private readonly requests = new Map<string, number>();
  private readonly _onDidChangeLoadState = this.own(new Emitter<AsyncDataTreeLoadStateEvent<T>>());
  private readonly _onDidError = this.own(new Emitter<AsyncDataTreeErrorEvent<T>>());
  private states = new Map<string, AsyncNodeState<T>>();
  private rootChildren: readonly string[] = [];
  private input: TInput | undefined;
  private generatedId = 0;
  private requestSequence = 0;
  private generation = 0;
  private disposed = false;

  readonly onPointer: Event<ObjectTreePointerEvent<T>>;
  readonly onDidDoubleClick: Event<ObjectTreePointerEvent<T>>;
  readonly onDidAccept: Event<ObjectTreeAcceptEvent<T>>;
  readonly onDidChangeFocus: Event<ObjectTreeFocusChangeEvent<T>>;
  readonly onDidChangeSelection: Event<ObjectTreeSelectionChangeEvent<T>>;
  readonly onDidChangeCollapseState: Event<ObjectTreeCollapseStateChangeEvent<T>>;
  readonly onDidChangeLoadState: Event<AsyncDataTreeLoadStateEvent<T>> = this._onDidChangeLoadState.event;
  readonly onDidError: Event<AsyncDataTreeErrorEvent<T>> = this._onDidError.event;

  constructor(private readonly dataSource: AsyncTreeDataSource<TInput, T>, private readonly options: AsyncDataTreeOptions<T>) {
    super();
    this.tree = this.own(new ObjectTree<T>({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      indent: options.indent,
      indentGuides: options.indentGuides,
      expandOnlyOnTwistieClick: options.expandOnlyOnTwistieClick,
      modelOptions: {
        defaultCollapseState: "collapsed",
        identityProvider: { getId: (element) => this.getId(element) },
        sorter: options.sorter,
        filter: options.filter,
      },
      onWillRender: options.onWillRender,
      renderElement: options.renderElement,
      renderTwistie: (element, state, container) => options.renderTwistie?.(element, { ...state, loading: this.isLoading(element) }, container),
    }));
    this.element = this.tree.element;
    this.onPointer = this.tree.onPointer;
    this.onDidDoubleClick = this.tree.onDidDoubleClick;
    this.onDidAccept = this.tree.onDidAccept;
    this.onDidChangeFocus = this.tree.onDidChangeFocus;
    this.onDidChangeSelection = this.tree.onDidChangeSelection;
    this.onDidChangeCollapseState = this.tree.onDidChangeCollapseState;
    this.own(this.tree.onDidChangeCollapseState(({ element, collapsed }) => {
      const state = this.states.get(this.getId(element));
      if (!collapsed && state?.hasChildren && state.children === undefined) void this.updateChildren(element).catch(() => undefined);
    }));
    this.defer(() => {
      this.disposed = true;
      this.generation += 1;
      this.requests.clear();
    });
  }

  getInput(): TInput | undefined { return this.input; }
  get focus(): T | undefined { return this.tree.focus; }
  get selection(): readonly T[] { return this.tree.selection; }

  async setInput(input: TInput | undefined): Promise<void> {
    const generation = ++this.generation;
    this.input = input;
    this.states.clear();
    this.rootChildren = [];
    this.requests.clear();
    this.render();
    if (input !== undefined) await this.loadChildren(input, undefined, generation);
  }

  async updateChildren(element: TInput | T = this.requireInput()): Promise<void> {
    const parentId = element === this.input ? undefined : this.getId(element as T);
    if (parentId !== undefined && !this.states.has(parentId)) throw new RangeError(`Unknown AsyncDataTree element: ${parentId}`);
    await this.loadChildren(element, parentId, this.generation);
  }

  collapse(element: T): boolean { return this.tree.collapse(this.getId(element)); }
  expand(element: T): boolean { return this.tree.expand(this.getId(element)); }
  expandTo(element: T): boolean { return this.tree.expandTo(this.getId(element)); }

  isLoading(element: T): boolean {
    return this.requests.has(this.getId(element));
  }

  private async loadChildren(parent: TInput | T, parentId: string | undefined, generation: number): Promise<void> {
    const requestKey = parentId ?? "__zeta_async_tree_root__";
    const request = ++this.requestSequence;
    this.requests.set(requestKey, request);
    this._onDidChangeLoadState.fire({ element: parentId === undefined ? undefined : parent as T, loading: true });
    if (parentId !== undefined) this.tree.model.rerender(parentId);
    try {
      const children = [...await this.dataSource.getChildren(parent)];
      if (!this.isCurrentRequest(requestKey, request, generation)) return;
      this.replaceChildren(parentId, children);
      this.render();
    } catch (error) {
      if (!this.isCurrentRequest(requestKey, request, generation)) return;
      this._onDidError.fire({ element: parentId === undefined ? undefined : parent as T, error });
      throw error;
    } finally {
      if (this.isCurrentRequest(requestKey, request, generation)) {
        this.requests.delete(requestKey);
        if (parentId !== undefined && this.tree.model.has(parentId)) this.tree.model.rerender(parentId);
        this._onDidChangeLoadState.fire({ element: parentId === undefined ? undefined : parent as T, loading: false });
      }
    }
  }

  private replaceChildren(parentId: string | undefined, elements: readonly T[]): void {
    const nextStates = new Map(this.states);
    const oldChildren = parentId === undefined ? this.rootChildren : nextStates.get(parentId)?.children ?? [];
    for (const id of oldChildren) removeStateSubtree(id, nextStates);
    const childIds: string[] = [];
    for (const element of elements) {
      const id = this.getId(element);
      if (childIds.includes(id) || nextStates.has(id)) throw new Error(`Duplicate tree node ID: ${id}`);
      childIds.push(id);
      nextStates.set(id, { id, element, parentId, hasChildren: this.dataSource.hasChildren(element), children: undefined });
    }
    if (parentId === undefined) this.rootChildren = childIds;
    else {
      const parent = nextStates.get(parentId);
      if (!parent) throw new RangeError(`Unknown AsyncDataTree parent: ${parentId}`);
      nextStates.set(parentId, { ...parent, children: childIds });
    }
    this.states = nextStates;
  }

  private render(): void {
    this.tree.setChildren(this.rootChildren.map((id) => this.toTreeElement(id)));
  }

  private toTreeElement(id: string): ObjectTreeElement<T> {
    const state = this.states.get(id);
    if (!state) throw new RangeError(`Unknown AsyncDataTree state: ${id}`);
    return {
      element: state.element,
      collapsible: state.hasChildren,
      collapsed: this.options.collapseByDefault?.(state.element),
      children: state.children?.map((childId) => this.toTreeElement(childId)),
    };
  }

  private getId(element: T): string {
    const external = this.options.identityProvider?.getId(element);
    if (external !== undefined) return external;
    let id = this.generatedIds.get(element);
    if (!id) {
      id = `async-data-tree-node-${++this.generatedId}`;
      this.generatedIds.set(element, id);
    }
    return id;
  }

  private isCurrentRequest(key: string, request: number, generation: number): boolean {
    return !this.disposed && generation === this.generation && this.requests.get(key) === request;
  }

  private requireInput(): TInput {
    if (this.input === undefined) throw new Error("AsyncDataTree input is not set");
    return this.input;
  }
}

function removeStateSubtree<T>(id: string, states: Map<string, AsyncNodeState<T>>): void {
  const state = states.get(id);
  if (!state) return;
  for (const child of state.children ?? []) removeStateSubtree(child, states);
  states.delete(id);
}
