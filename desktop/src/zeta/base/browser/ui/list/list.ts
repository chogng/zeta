import {
  addDisposableListener,
  isHTMLElement,
  stopEvent,
} from "../../dom.js";
import {
  setAriaAttribute,
  setRole,
} from "../aria/aria.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface ListOptions<T> {
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly loopNavigation?: boolean;
  readonly renderItem: (item: T, index: number) => HTMLElement;
}

export interface ListActiveChangeEvent<T> {
  readonly item: T | undefined;
  readonly index: number;
  readonly rowId: string | undefined;
}

export interface ListAcceptEvent<T> {
  readonly item: T;
  readonly index: number;
  readonly browserEvent: MouseEvent | undefined;
}

/**
 * A single-focus list foundation with rendering, navigation, mouse acceptance,
 * and listbox accessibility. Product-specific filtering stays with callers.
 */
export class List<T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly renderItem: ListOptions<T>["renderItem"];
  private readonly loopNavigation: boolean;
  private readonly _onDidChangeActive =
    this.own(new Emitter<ListActiveChangeEvent<T>>());
  private readonly _onDidAccept = this.own(new Emitter<ListAcceptEvent<T>>());
  private _items: readonly T[] = [];
  private _activeIndex = -1;

  readonly onDidChangeActive: Event<ListActiveChangeEvent<T>> =
    this._onDidChangeActive.event;
  readonly onDidAccept: Event<ListAcceptEvent<T>> =
    this._onDidAccept.event;

  constructor(options: ListOptions<T>) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    this.renderItem = options.renderItem;
    this.loopNavigation = options.loopNavigation ?? true;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-list";
    this.element.id = `zeta-list-${listSequence++}`;
    setRole(this.element, "listbox");
    if (options.ariaLabel) {
      setAriaAttribute(this.element, "label", options.ariaLabel);
    }
    this.defer(() => this.element.remove());
    this.own(addDisposableListener(
      this.element,
      "mousemove",
      (event: MouseEvent) => {
        const index = this.rowIndexFromEvent(event);
        if (index !== undefined) this.setActiveIndex(index);
      },
    ));
    this.own(addDisposableListener(
      this.element,
      "mousedown",
      (event: MouseEvent) => {
        if (this.rowIndexFromEvent(event) !== undefined) {
          stopEvent(event);
        }
      },
    ));
    this.own(addDisposableListener(
      this.element,
      "click",
      (event: MouseEvent) => {
        const index = this.rowIndexFromEvent(event);
        if (index === undefined) return;
        this.setActiveIndex(index);
        this.acceptActive(event);
      },
    ));
  }

  get items(): readonly T[] {
    return this._items;
  }

  set items(items: readonly T[]) {
    this._items = [...items];
    const rows = this._items.map((item, index) => {
      const row = this.element.ownerDocument.createElement("div");
      row.className = "zeta-list-row";
      row.id = `${this.element.id}-item-${index}`;
      row.dataset.index = String(index);
      setRole(row, "option");
      setAriaAttribute(row, "selected", false);
      row.append(this.renderItem(item, index));
      return row;
    });
    this.element.replaceChildren(...rows);
    this._activeIndex = rows.length > 0 ? 0 : -1;
    this.syncActiveRows();
  }

  get activeIndex(): number {
    return this._activeIndex;
  }

  get activeItem(): T | undefined {
    return this._items[this._activeIndex];
  }

  setActiveIndex(index: number): void {
    if (!Number.isInteger(index) || index < 0 || index >= this._items.length) {
      return;
    }
    if (this._activeIndex === index) return;
    this._activeIndex = index;
    this.syncActiveRows();
  }

  focusNext(): void {
    this.moveActive(1);
  }

  focusPrevious(): void {
    this.moveActive(-1);
  }

  acceptActive(browserEvent?: MouseEvent): void {
    const item = this.activeItem;
    if (item === undefined) return;
    this._onDidAccept.fire({
      item,
      index: this._activeIndex,
      browserEvent,
    });
  }

  private moveActive(delta: number): void {
    const length = this._items.length;
    if (length === 0) return;
    const candidate = this._activeIndex + delta;
    const next = this.loopNavigation
      ? (candidate + length) % length
      : Math.max(0, Math.min(candidate, length - 1));
    this.setActiveIndex(next);
  }

  private syncActiveRows(): void {
    const rows = this.element.querySelectorAll<HTMLElement>(
      ":scope > .zeta-list-row",
    );
    rows.forEach((row, index) => {
      const active = index === this._activeIndex;
      row.classList.toggle("is-active", active);
      setAriaAttribute(row, "selected", active);
      if (active) row.scrollIntoView?.({ block: "nearest" });
    });
    const activeRow = rows[this._activeIndex];
    this._onDidChangeActive.fire({
      item: this.activeItem,
      index: this._activeIndex,
      rowId: activeRow?.id,
    });
  }

  private rowIndexFromEvent(event: MouseEvent): number | undefined {
    if (!isHTMLElement(event.target)) return undefined;
    const row = event.target.closest<HTMLElement>(".zeta-list-row");
    if (!row || row.parentElement !== this.element) return undefined;
    const index = Number(row.dataset.index);
    return Number.isInteger(index) ? index : undefined;
  }
}

let listSequence = 1;
