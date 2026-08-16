import { addDisposableListener, isHTMLElement, stopEvent } from "../../dom.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export const TreeVisibility = Object.freeze({
  Hidden: "hidden",
  Visible: "visible",
  Recurse: "recurse",
} as const);

export type TreeVisibility = typeof TreeVisibility[keyof typeof TreeVisibility];
export type TreeFilterResult = boolean | TreeVisibility;

export interface TreeFilter<T> {
  filter(element: T, parentVisibility: TreeVisibility): TreeFilterResult;
}

export interface TreeSorter<T> {
  compare(left: T, right: T): number;
}

export type TreeIndentGuides = "none" | "onHover" | "always";

export interface TreeTwistieState {
  readonly collapsible: boolean;
  readonly expanded: boolean;
}

export interface TreeOptions<T> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly indent?: number;
  readonly indentGuides?: TreeIndentGuides;
  readonly getId: (element: T) => string;
  readonly getChildren: (element: T) => readonly T[] | undefined;
  readonly isCollapsible: (element: T) => boolean;
  readonly isExpanded: (element: T) => boolean;
  readonly renderElement: (element: T) => HTMLElement;
  readonly renderTwistie?: (element: T, state: TreeTwistieState, container: HTMLSpanElement) => void;
}

export interface TreeActivateEvent<T> {
  readonly element: T;
  readonly browserEvent: MouseEvent | KeyboardEvent;
}

interface RenderedTreeElement<T> {
  readonly element: T;
  readonly parentId: string | undefined;
  readonly row: HTMLButtonElement;
}

/**
 * Accessible hierarchical control that owns tree geometry and interaction.
 *
 * Callers provide domain elements and project their current expanded state.
 * Activating a row asks the caller to perform the domain action; assigning
 * `items` again renders the resulting state without moving ownership into the
 * base layer.
 */
export class Tree<T> extends DisposableOwner {
  readonly element: HTMLUListElement;
  private readonly options: TreeOptions<T>;
  private readonly rendered = new Map<string, RenderedTreeElement<T>>();
  private readonly visibleIds: string[] = [];
  private readonly _onDidActivate = this.own(new Emitter<TreeActivateEvent<T>>());
  private _items: readonly T[] = [];
  private focusedId: string | undefined;

  readonly onDidActivate: Event<TreeActivateEvent<T>> = this._onDidActivate.event;

  constructor(options: TreeOptions<T>) {
    super();
    this.options = options;
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("ul");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-tree zeta-tree-indent-guides-${options.indentGuides ?? "none"}`;
    element.setAttribute("role", "tree");
    if (options.ariaLabel) element.setAttribute("aria-label", options.ariaLabel);
    const indent = options.indent;
    if (indent !== undefined && (!Number.isFinite(indent) || indent < 4 || indent > 40)) {
      throw new RangeError("Tree indent must be between 4 and 40 pixels");
    }
    if (indent !== undefined) element.style.setProperty("--zeta-tree-indent", `${indent}px`);
    this.own(addDisposableListener(element, "click", (event) => this.onClick(event)));
    this.own(addDisposableListener(element, "focusin", (event) => this.onFocusIn(event)));
    this.own(addDisposableListener(element, "keydown", (event) => this.onKeyDown(event)));
  }

  get items(): readonly T[] {
    return this._items;
  }

  set items(items: readonly T[]) {
    this._items = [...items];
    this.render();
  }

  private render(): void {
    const restoreFocus = this.element.contains(this.element.ownerDocument.activeElement);
    this.rendered.clear();
    this.visibleIds.length = 0;
    const nodes = this._items.map((element, index) => this.renderElement(element, undefined, 1, index, this._items.length));
    this.element.replaceChildren(...nodes);
    if (!this.focusedId || !this.rendered.has(this.focusedId)) {
      this.focusedId = this.visibleIds[0];
    }
    this.syncTabStops();
    if (restoreFocus && this.focusedId) this.rendered.get(this.focusedId)?.row.focus();
  }

  private renderElement(element: T, parentId: string | undefined, level: number, index: number, setSize: number): HTMLLIElement {
    const id = this.options.getId(element);
    if (!id) throw new TypeError("Tree element IDs must not be empty");
    if (this.rendered.has(id)) throw new Error(`Duplicate tree element ID: ${id}`);
    const document = this.element.ownerDocument;
    const node = document.createElement("li");
    node.className = "zeta-tree-node";
    node.setAttribute("role", "none");
    const row = document.createElement("button");
    row.type = "button";
    row.className = "zeta-tree-row";
    row.dataset.treeId = id;
    row.setAttribute("role", "treeitem");
    row.setAttribute("aria-level", String(level));
    row.setAttribute("aria-posinset", String(index + 1));
    row.setAttribute("aria-setsize", String(setSize));
    row.style.paddingLeft = treeRowPadding(level);
    const collapsible = this.options.isCollapsible(element);
    const expanded = collapsible && this.options.isExpanded(element);
    row.classList.toggle("collapsible", collapsible);
    row.classList.toggle("expanded", expanded);
    row.classList.toggle("collapsed", collapsible && !expanded);
    if (collapsible) row.setAttribute("aria-expanded", String(expanded));
    const indentGuides = document.createElement("span");
    indentGuides.className = "zeta-tree-indent";
    indentGuides.setAttribute("aria-hidden", "true");
    for (let guideIndex = 1; guideIndex < level; guideIndex += 1) {
      const guide = document.createElement("span");
      guide.className = "zeta-tree-indent-guide";
      indentGuides.append(guide);
    }
    const twistie = document.createElement("span");
    twistie.className = "zeta-tree-twistie";
    twistie.setAttribute("aria-hidden", "true");
    this.options.renderTwistie?.(element, { collapsible, expanded }, twistie);
    const contents = document.createElement("span");
    contents.className = "zeta-tree-contents";
    contents.append(this.options.renderElement(element));
    row.append(indentGuides, twistie, contents);
    node.append(row);
    this.rendered.set(id, { element, parentId, row });
    this.visibleIds.push(id);
    const children = expanded ? this.options.getChildren(element) ?? [] : [];
    if (children.length > 0) {
      const group = document.createElement("ul");
      group.className = "zeta-tree-group";
      group.setAttribute("role", "group");
      for (let childIndex = 0; childIndex < children.length; childIndex += 1) {
        group.append(this.renderElement(children[childIndex]!, id, level + 1, childIndex, children.length));
      }
      node.append(group);
    }
    return node;
  }

  private onClick(event: MouseEvent): void {
    const rendered = this.renderedFromTarget(event.target);
    if (!rendered) return;
    this.focusRow(rendered);
    this._onDidActivate.fire({ element: rendered.element, browserEvent: event });
  }

  private onFocusIn(event: FocusEvent): void {
    const rendered = this.renderedFromTarget(event.target);
    if (rendered) this.focusRow(rendered, false);
  }

  private onKeyDown(event: KeyboardEvent): void {
    const rendered = this.renderedFromTarget(event.target);
    if (!rendered) return;
    const index = this.visibleIds.indexOf(this.options.getId(rendered.element));
    if (event.key === "ArrowDown") {
      stopEvent(event);
      this.focusByIndex(index + 1);
      return;
    }
    if (event.key === "ArrowUp") {
      stopEvent(event);
      this.focusByIndex(index - 1);
      return;
    }
    if (event.key === "Home") {
      stopEvent(event);
      this.focusByIndex(0);
      return;
    }
    if (event.key === "End") {
      stopEvent(event);
      this.focusByIndex(this.visibleIds.length - 1);
      return;
    }
    const collapsible = this.options.isCollapsible(rendered.element);
    const expanded = collapsible && this.options.isExpanded(rendered.element);
    if (event.key === "ArrowRight" && collapsible) {
      stopEvent(event);
      if (!expanded) {
        this._onDidActivate.fire({ element: rendered.element, browserEvent: event });
      } else if ((this.options.getChildren(rendered.element)?.length ?? 0) > 0) {
        this.focusByIndex(index + 1);
      }
      return;
    }
    if (event.key === "ArrowLeft") {
      if (expanded) {
        stopEvent(event);
        this._onDidActivate.fire({ element: rendered.element, browserEvent: event });
      } else if (rendered.parentId) {
        stopEvent(event);
        const parent = this.rendered.get(rendered.parentId);
        if (parent) this.focusRow(parent);
      }
    }
  }

  private renderedFromTarget(target: EventTarget | null): RenderedTreeElement<T> | undefined {
    if (!isHTMLElement(target)) return undefined;
    const row = target.closest<HTMLButtonElement>(".zeta-tree-row");
    if (!row || !this.element.contains(row)) return undefined;
    return this.rendered.get(row.dataset.treeId ?? "");
  }

  private focusByIndex(index: number): void {
    const bounded = Math.max(0, Math.min(index, this.visibleIds.length - 1));
    const id = this.visibleIds[bounded];
    if (!id) return;
    const rendered = this.rendered.get(id);
    if (rendered) this.focusRow(rendered);
  }

  private focusRow(rendered: RenderedTreeElement<T>, moveDomFocus = true): void {
    this.focusedId = this.options.getId(rendered.element);
    this.syncTabStops();
    if (moveDomFocus) rendered.row.focus();
  }

  private syncTabStops(): void {
    for (const [id, { row }] of this.rendered) row.tabIndex = id === this.focusedId ? 0 : -1;
  }
}

function treeRowPadding(level: number): string {
  if (level === 1) return "8px";
  return `calc(8px + ${Array.from({ length: level - 1 }, () => "var(--zeta-tree-indent, 14px)").join(" + ")})`;
}
