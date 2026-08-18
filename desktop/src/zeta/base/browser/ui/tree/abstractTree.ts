import { addDisposableListener, isNode, stopEvent } from "../../dom.js";
import { List } from "../list/list.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import type { AbstractTreeNode, TreeAcceptEvent, TreeActivateEvent, TreeCollapseRequestEvent, TreeFocusChangeEvent, TreeIndentGuides, TreePointerEvent, TreePointerTarget, TreeSelectionChangeEvent, TreeTwistieState } from "./tree.js";

export interface AbstractTreeOptions<T, TNode extends AbstractTreeNode<T>> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly expandOnlyOnTwistieClick?: boolean | ((element: TNode) => boolean);
  readonly renderElement: (element: TNode) => HTMLElement;
  readonly renderTwistie?: (element: TNode, state: TreeTwistieState, container: HTMLSpanElement) => void;
}

/**
 * Projects model-owned tree nodes through the shared flat `List` foundation.
 *
 * Implementations provide an already-flattened visible-node sequence. This
 * layer owns tree row semantics and interaction, but never reconstructs or
 * mutates the hierarchy itself.
 */
export class AbstractTree<T, TNode extends AbstractTreeNode<T>> extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly list: List<TNode>;
  private readonly options: AbstractTreeOptions<T, TNode>;
  private readonly _onPointer = this.own(new Emitter<TreePointerEvent<TNode>>());
  private readonly _onDidDoubleClick = this.own(new Emitter<TreePointerEvent<TNode>>());
  private readonly _onDidAccept = this.own(new Emitter<TreeAcceptEvent<TNode>>());
  private readonly _onDidChangeFocus = this.own(new Emitter<TreeFocusChangeEvent<TNode>>());
  private readonly _onDidChangeSelection = this.own(new Emitter<TreeSelectionChangeEvent<TNode>>());
  private readonly _onDidRequestCollapseChange = this.own(new Emitter<TreeCollapseRequestEvent<TNode>>());
  private readonly _onDidActivate = this.own(new Emitter<TreeActivateEvent<TNode>>());

  readonly onPointer: Event<TreePointerEvent<TNode>> = this._onPointer.event;
  readonly onDidDoubleClick: Event<TreePointerEvent<TNode>> = this._onDidDoubleClick.event;
  readonly onDidAccept: Event<TreeAcceptEvent<TNode>> = this._onDidAccept.event;
  readonly onDidChangeFocus: Event<TreeFocusChangeEvent<TNode>> = this._onDidChangeFocus.event;
  readonly onDidChangeSelection: Event<TreeSelectionChangeEvent<TNode>> = this._onDidChangeSelection.event;
  readonly onDidRequestCollapseChange: Event<TreeCollapseRequestEvent<TNode>> = this._onDidRequestCollapseChange.event;
  /** @deprecated Prefer the semantic pointer, accept, and collapse events. */
  readonly onDidActivate: Event<TreeActivateEvent<TNode>> = this._onDidActivate.event;

  constructor(options: AbstractTreeOptions<T, TNode>) {
    super();
    this.options = options;
    validateIndent(options.indent);
    this.list = this.own(new List<TNode>({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      role: "tree",
      loopNavigation: false,
      keyboardNavigation: true,
      focusOnMouseMove: false,
      acceptOnClick: false,
      domFocusable: true,
      getId: (node) => node.id,
      accessibilityProvider: {
        getRole: () => "treeitem",
        getAriaLevel: (node) => node.depth,
        getAriaSetSize: (node) => node.visibleChildrenCount,
        getAriaPosInSet: (node) => node.visibleChildIndex + 1,
        isExpanded: (node) => node.collapsible ? !node.collapsed : undefined,
      },
      renderItem: (node, _index, row) => this.renderRow(node, row),
    }));
    this.element = this.list.element;
    this.element.classList.add("zeta-tree", `zeta-tree-indent-guides-${options.indentGuides ?? "none"}`);
    if (options.indent !== undefined) this.element.style.setProperty("--zeta-tree-indent", `${options.indent}px`);
    this.own(this.list.onPointer((event) => this.onListPointer(event.item, event.browserEvent)));
    this.own(this.list.onDidDoubleClick((event) => this.onListDoubleClick(event.item, event.browserEvent)));
    this.own(this.list.onDidChangeFocus(({ item, browserEvent }) => this._onDidChangeFocus.fire({ element: item, browserEvent })));
    this.own(this.list.onDidChangeSelection(({ items, browserEvent }) => this._onDidChangeSelection.fire({ elements: items, browserEvent })));
    this.own(addDisposableListener(this.element, "keydown", (event: KeyboardEvent) => this.onKeyDown(event)));
  }

  get items(): readonly TNode[] { return this.list.items; }
  set items(items: readonly TNode[]) { this.list.items = items; }
  get focus(): TNode | undefined { return this.list.activeItem; }
  get selection(): readonly TNode[] { return this.list.selection; }

  setFocus(id: string, browserEvent?: UIEvent): void {
    const index = this.items.findIndex((node) => node.id === id);
    if (index >= 0) this.list.setActiveIndex(index, browserEvent);
  }

  setSelection(ids: readonly string[], browserEvent?: UIEvent): void {
    const selected = new Set(ids);
    this.list.setSelection(this.items.flatMap((node, index) => selected.has(node.id) ? [index] : []), browserEvent);
  }

  private renderRow(node: TNode, row: HTMLDivElement): HTMLElement {
    const document = row.ownerDocument;
    row.classList.add("zeta-tree-row");
    row.dataset.treeId = node.id;
    row.classList.toggle("collapsible", node.collapsible);
    row.classList.toggle("expanded", node.collapsible && !node.collapsed);
    row.classList.toggle("collapsed", node.collapsible && node.collapsed);
    row.style.paddingLeft = treeRowPadding(node.depth);
    const inner = document.createElement("span");
    inner.className = "zeta-tree-row-inner";
    const indent = document.createElement("span");
    indent.className = "zeta-tree-indent";
    indent.setAttribute("aria-hidden", "true");
    for (let index = 1; index < node.depth; index += 1) {
      const guide = document.createElement("span");
      guide.className = "zeta-tree-indent-guide";
      indent.append(guide);
    }
    const twistie = document.createElement("span");
    twistie.className = "zeta-tree-twistie";
    twistie.setAttribute("aria-hidden", "true");
    this.options.renderTwistie?.(node, { collapsible: node.collapsible, expanded: node.collapsible && !node.collapsed }, twistie);
    const contents = document.createElement("span");
    contents.className = "zeta-tree-contents";
    contents.append(this.options.renderElement(node));
    inner.append(indent, twistie, contents);
    return inner;
  }

  private onListPointer(node: TNode, browserEvent: MouseEvent): void {
    const target = this.pointerTarget(browserEvent);
    if (target === "twistie") {
      if (browserEvent.button === 0) this.requestCollapseChange(node, browserEvent);
      return;
    }
    if (browserEvent.button === 0 && browserEvent.detail !== 2 && !this.expandOnlyOnTwistieClick(node)) this.requestCollapseChange(node, browserEvent);
    const event = { element: node, target, browserEvent } as const;
    this._onPointer.fire(event);
    this._onDidActivate.fire({ element: node, browserEvent });
  }

  private onListDoubleClick(node: TNode, browserEvent: MouseEvent): void {
    const target = this.pointerTarget(browserEvent);
    if (target === "twistie") return;
    if (browserEvent.button === 0 && this.expandOnlyOnTwistieClick(node)) this.requestCollapseChange(node, browserEvent);
    this._onDidDoubleClick.fire({ element: node, target, browserEvent });
  }

  private onKeyDown(event: KeyboardEvent): void {
    const node = this.list.activeItem;
    if (!node) return;
    if (event.key === "ArrowRight" && node.collapsible) {
      stopEvent(event);
      if (node.collapsed) this._onDidRequestCollapseChange.fire({ element: node, expanded: true, browserEvent: event });
      else if (node.children.length > 0) {
        const childIndex = this.list.activeIndex + 1;
        this.list.setActiveIndex(childIndex, event);
        this.list.setSelection([childIndex], event);
      }
      return;
    }
    if (event.key === "ArrowLeft") {
      if (node.collapsible && !node.collapsed) {
        stopEvent(event);
        this._onDidRequestCollapseChange.fire({ element: node, expanded: false, browserEvent: event });
      } else if (node.parent) {
        stopEvent(event);
        this.setFocus(node.parent.id, event);
        this.setSelection([node.parent.id], event);
      }
      return;
    }
    if (event.key !== "Enter" && event.key !== " ") return;
    stopEvent(event);
    this.list.setSelection([this.list.activeIndex], event);
    this._onDidAccept.fire({ element: node, browserEvent: event });
    this._onDidActivate.fire({ element: node, browserEvent: event });
  }

  private requestCollapseChange(node: TNode, browserEvent: MouseEvent): void {
    if (node.collapsible) this._onDidRequestCollapseChange.fire({ element: node, expanded: node.collapsed, browserEvent });
  }

  private expandOnlyOnTwistieClick(node: TNode): boolean {
    const value = this.options.expandOnlyOnTwistieClick;
    return typeof value === "function" ? value(node) : value ?? true;
  }

  private pointerTarget(event: MouseEvent): TreePointerTarget {
    if (!isNode(event.target) || event.target.nodeType !== 1) return "contents";
    return (event.target as Element).closest(".zeta-tree-twistie") ? "twistie" : "contents";
  }
}

function validateIndent(indent: number | undefined): void {
  if (indent !== undefined && (!Number.isFinite(indent) || indent < 4 || indent > 40)) throw new RangeError("Tree indent must be between 4 and 40 pixels");
}

function treeRowPadding(level: number): string {
  return level === 1 ? "8px" : `calc(8px + ${Array.from({ length: level - 1 }, () => "var(--zeta-tree-indent, 14px)").join(" + ")})`;
}
