import { addDisposableListener, isNode, stopEvent } from "../../dom.js";
import { setAriaAttribute, setRole } from "../aria/aria.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface ListAccessibilityProvider<T> {
  readonly getRole?: (item: T) => "option" | "treeitem";
  readonly getAriaLabel?: (item: T) => string | undefined;
  readonly getAriaLevel?: (item: T) => number | undefined;
  readonly getAriaSetSize?: (item: T) => number | undefined;
  readonly getAriaPosInSet?: (item: T) => number | undefined;
  readonly isExpanded?: (item: T) => boolean | undefined;
}

export interface ListOptions<T> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly role?: "listbox" | "tree";
  readonly loopNavigation?: boolean;
  readonly keyboardNavigation?: boolean;
  readonly focusOnMouseMove?: boolean;
  readonly acceptOnClick?: boolean;
  readonly domFocusable?: boolean;
  readonly getId?: (item: T) => string;
  readonly accessibilityProvider?: ListAccessibilityProvider<T>;
  readonly renderItem: (item: T, index: number, row: HTMLDivElement) => HTMLElement;
}

export interface ListActiveChangeEvent<T> {
  readonly item: T | undefined;
  readonly index: number;
  readonly rowId: string | undefined;
  readonly browserEvent?: UIEvent;
}

export interface ListSelectionChangeEvent<T> {
  readonly items: readonly T[];
  readonly indexes: readonly number[];
  readonly browserEvent?: UIEvent;
}

export interface ListPointerEvent<T> {
  readonly item: T;
  readonly index: number;
  readonly browserEvent: MouseEvent;
}

export interface ListAcceptEvent<T> {
  readonly item: T;
  readonly index: number;
  readonly browserEvent: MouseEvent | KeyboardEvent | undefined;
}

/** Flat row foundation shared by listbox and tree projections. */
export class List<T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly options: ListOptions<T>;
  private readonly loopNavigation: boolean;
  private readonly _onDidChangeActive = this.own(new Emitter<ListActiveChangeEvent<T>>());
  private readonly _onDidChangeSelection = this.own(new Emitter<ListSelectionChangeEvent<T>>());
  private readonly _onPointer = this.own(new Emitter<ListPointerEvent<T>>());
  private readonly _onDidDoubleClick = this.own(new Emitter<ListPointerEvent<T>>());
  private readonly _onDidAccept = this.own(new Emitter<ListAcceptEvent<T>>());
  private _items: readonly T[] = [];
  private _activeIndex = -1;
  private _selectionIndexes: readonly number[] = [];

  readonly onDidChangeActive: Event<ListActiveChangeEvent<T>> = this._onDidChangeActive.event;
  readonly onDidChangeFocus: Event<ListActiveChangeEvent<T>> = this._onDidChangeActive.event;
  readonly onDidChangeSelection: Event<ListSelectionChangeEvent<T>> = this._onDidChangeSelection.event;
  readonly onPointer: Event<ListPointerEvent<T>> = this._onPointer.event;
  readonly onDidDoubleClick: Event<ListPointerEvent<T>> = this._onDidDoubleClick.event;
  readonly onDidAccept: Event<ListAcceptEvent<T>> = this._onDidAccept.event;

  constructor(options: ListOptions<T>) {
    super();
    this.options = options;
    this.loopNavigation = options.loopNavigation ?? true;
    const ownerDocument = options.ownerDocument ?? document;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-list";
    this.element.id = `zeta-list-${listSequence++}`;
    setRole(this.element, options.role ?? "listbox");
    if (options.ariaLabel) setAriaAttribute(this.element, "label", options.ariaLabel);
    if (options.domFocusable === true) this.element.tabIndex = 0;
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(this.element, "mousemove", (event: MouseEvent) => {
      if (options.focusOnMouseMove === false) return;
      const index = this.rowIndexFromEvent(event);
      if (index !== undefined) this.setActiveIndex(index, event);
    }));
    this.own(addDisposableListener(this.element, "mousedown", (event: MouseEvent) => {
      if (this.rowIndexFromEvent(event) !== undefined) stopEvent(event);
    }));
    this.own(addDisposableListener(this.element, "click", (event: MouseEvent) => this.onClick(event)));
    this.own(addDisposableListener(this.element, "auxclick", (event: MouseEvent) => this.onAuxClick(event)));
    this.own(addDisposableListener(this.element, "dblclick", (event: MouseEvent) => this.onDoubleClick(event)));
    if (options.keyboardNavigation === true) this.own(addDisposableListener(this.element, "keydown", (event: KeyboardEvent) => this.onKeyDown(event)));
  }

  get items(): readonly T[] { return this._items; }

  set items(items: readonly T[]) {
    const focusedId = this.activeItem === undefined ? undefined : this.itemId(this.activeItem, this._activeIndex);
    const selectedIds = this._selectionIndexes.map((index) => this.itemId(this._items[index], index));
    this._items = [...items];
    const seen = new Set<string>();
    const rows = this._items.map((item, index) => {
      const itemId = this.itemId(item, index);
      if (seen.has(itemId)) throw new TypeError(`Duplicate List item ID: ${itemId}`);
      seen.add(itemId);
      const row = this.element.ownerDocument.createElement("div");
      row.className = "zeta-list-row";
      row.id = `${this.element.id}-item-${encodeURIComponent(itemId)}`;
      row.dataset.index = String(index);
      row.dataset.listId = itemId;
      const accessibility = this.options.accessibilityProvider;
      setRole(row, accessibility?.getRole?.(item) ?? (this.options.role === "tree" ? "treeitem" : "option"));
      setAriaAttribute(row, "selected", false);
      this.setNumericAria(row, "aria-level", accessibility?.getAriaLevel?.(item));
      this.setNumericAria(row, "aria-setsize", accessibility?.getAriaSetSize?.(item));
      this.setNumericAria(row, "aria-posinset", accessibility?.getAriaPosInSet?.(item));
      const expanded = accessibility?.isExpanded?.(item);
      if (expanded !== undefined) row.setAttribute("aria-expanded", String(expanded));
      const ariaLabel = accessibility?.getAriaLabel?.(item);
      if (ariaLabel) row.setAttribute("aria-label", ariaLabel);
      row.append(this.options.renderItem(item, index, row));
      return row;
    });
    this.element.replaceChildren(...rows);
    const nextActive = focusedId === undefined ? -1 : this.indexOfId(focusedId);
    this._activeIndex = nextActive >= 0 ? nextActive : rows.length > 0 ? 0 : -1;
    this._selectionIndexes = selectedIds.map((id) => this.indexOfId(id)).filter((index) => index >= 0);
    this.syncRows();
    const nextFocusedId = this.activeItem === undefined ? undefined : this.itemId(this.activeItem, this._activeIndex);
    const nextSelectedIds = this._selectionIndexes.map((index) => this.itemId(this._items[index], index));
    if (focusedId !== nextFocusedId) this.emitFocus(undefined);
    if (!sameStrings(selectedIds, nextSelectedIds)) this.emitSelection(undefined);
  }

  get activeIndex(): number { return this._activeIndex; }
  get activeItem(): T | undefined { return this._items[this._activeIndex]; }
  get selection(): readonly T[] { return this._selectionIndexes.map((index) => this._items[index]!).filter((item) => item !== undefined); }

  setActiveIndex(index: number, browserEvent?: UIEvent): void {
    if (!Number.isInteger(index) || index < 0 || index >= this._items.length || this._activeIndex === index) return;
    this._activeIndex = index;
    this.syncRows();
    this.emitFocus(browserEvent);
  }

  setSelection(indexes: readonly number[], browserEvent?: UIEvent): void {
    const normalized = [...new Set(indexes)].filter((index) => Number.isInteger(index) && index >= 0 && index < this._items.length);
    if (sameNumbers(this._selectionIndexes, normalized)) return;
    this._selectionIndexes = normalized;
    this.syncRows();
    this.emitSelection(browserEvent);
  }

  focusNext(browserEvent?: UIEvent): void { this.moveActive(1, browserEvent); }
  focusPrevious(browserEvent?: UIEvent): void { this.moveActive(-1, browserEvent); }
  domFocus(): void { this.element.focus(); }

  acceptActive(browserEvent?: MouseEvent | KeyboardEvent): void {
    const item = this.activeItem;
    if (item !== undefined) this._onDidAccept.fire({ item, index: this._activeIndex, browserEvent });
  }

  row(index: number): HTMLElement | undefined {
    return this.element.querySelector<HTMLElement>(`:scope > .zeta-list-row[data-index="${index}"]`) ?? undefined;
  }

  private onClick(event: MouseEvent): void {
    const index = this.rowIndexFromEvent(event);
    if (index === undefined) return;
    this.domFocus();
    this.setActiveIndex(index, event);
    this.setSelection([index], event);
    this._onPointer.fire({ item: this._items[index]!, index, browserEvent: event });
    if (this.options.acceptOnClick !== false) this.acceptActive(event);
  }

  private onAuxClick(event: MouseEvent): void {
    if (event.button !== 1) return;
    const index = this.rowIndexFromEvent(event);
    if (index === undefined) return;
    this.setActiveIndex(index, event);
    this.setSelection([index], event);
    this._onPointer.fire({ item: this._items[index]!, index, browserEvent: event });
  }

  private onDoubleClick(event: MouseEvent): void {
    const index = this.rowIndexFromEvent(event);
    if (index !== undefined) this._onDidDoubleClick.fire({ item: this._items[index]!, index, browserEvent: event });
  }

  private onKeyDown(event: KeyboardEvent): void {
    let index: number | undefined;
    if (event.key === "ArrowDown") index = this.nextIndex(1);
    else if (event.key === "ArrowUp") index = this.nextIndex(-1);
    else if (event.key === "Home") index = this._items.length > 0 ? 0 : undefined;
    else if (event.key === "End") index = this._items.length > 0 ? this._items.length - 1 : undefined;
    if (index === undefined) return;
    stopEvent(event);
    this.setActiveIndex(index, event);
    this.setSelection([index], event);
  }

  private moveActive(delta: number, browserEvent?: UIEvent): void {
    const next = this.nextIndex(delta);
    if (next !== undefined) this.setActiveIndex(next, browserEvent);
  }

  private nextIndex(delta: number): number | undefined {
    const length = this._items.length;
    if (length === 0) return undefined;
    const candidate = this._activeIndex + delta;
    return this.loopNavigation ? (candidate + length) % length : Math.max(0, Math.min(candidate, length - 1));
  }

  private syncRows(): void {
    const rows = this.element.querySelectorAll<HTMLElement>(":scope > .zeta-list-row");
    rows.forEach((row, index) => {
      const focused = index === this._activeIndex;
      const selected = this._selectionIndexes.includes(index);
      row.classList.toggle("focused", focused);
      row.classList.toggle("is-active", focused);
      row.classList.toggle("selected", selected);
      setAriaAttribute(row, "selected", selected);
    });
    const activeRow = rows[this._activeIndex];
    if (activeRow) {
      this.element.setAttribute("aria-activedescendant", activeRow.id);
      activeRow.scrollIntoView?.({ block: "nearest" });
    } else this.element.removeAttribute("aria-activedescendant");
  }

  private emitFocus(browserEvent?: UIEvent): void {
    this._onDidChangeActive.fire({ item: this.activeItem, index: this._activeIndex, rowId: this.row(this._activeIndex)?.id, browserEvent });
  }

  private emitSelection(browserEvent?: UIEvent): void {
    this._onDidChangeSelection.fire({ items: this.selection, indexes: this._selectionIndexes, browserEvent });
  }

  private rowIndexFromEvent(event: MouseEvent): number | undefined {
    if (!isNode(event.target) || event.target.nodeType !== 1) return undefined;
    const row = (event.target as Element).closest<HTMLElement>(".zeta-list-row");
    if (!row || row.parentElement !== this.element) return undefined;
    const index = Number(row.dataset.index);
    return Number.isInteger(index) ? index : undefined;
  }

  private itemId(item: T | undefined, index: number): string {
    return item === undefined ? String(index) : this.options.getId?.(item) ?? String(index);
  }

  private indexOfId(id: string): number {
    return this._items.findIndex((item, index) => this.itemId(item, index) === id);
  }

  private setNumericAria(row: HTMLElement, name: string, value: number | undefined): void {
    if (value !== undefined) row.setAttribute(name, String(value));
  }
}

function sameNumbers(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function sameStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

let listSequence = 1;
