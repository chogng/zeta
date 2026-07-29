import "./tablist.css";
import { addDisposableListener } from "../../dom.js";
import type { Icon } from "../../../common/icon.js";
import type { IAction } from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { ActionBar } from "../actionbar/actionbar.js";
import { ActionViewItem } from "../actionbar/actionViewItems.js";
import { IconLabel } from "../iconlabel/iconlabel.js";
import { ScrollableElement } from "../scrollbar/scrollableElement.js";

/** Accessible actions rendered after one Tab's label content. */
export interface TabListActions {
  readonly ariaLabel: string;
  readonly items: readonly IAction[];
}

/** One selectable content identity rendered by a TabList. */
export interface TabListItem<T> {
  readonly id: string;
  readonly value: T;
  readonly label: string;
  readonly ariaLabel?: string;
  readonly tooltip?: string;
  readonly icon?: Icon;
  readonly tabId: string;
  readonly panelId?: string;
  readonly actions?: TabListActions;
}

/** Construction inputs for a manually activated horizontal TabList. */
export interface TabListOptions<T> {
  readonly ownerDocument: Document;
  readonly ariaLabel: string;
  readonly onActivate: (value: T) => void;
  readonly onDelete?: (value: T) => void;
}

/**
 * Domain-neutral tab semantics built on the shared roving-focus engine.
 *
 * Arrow keys move focus without changing selection. Callers own content
 * activation and panel lifetimes, then provide the resulting selected ID.
 */
export class TabList<T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #actionBar: ActionBar;
  readonly #scrollable: ScrollableElement;
  readonly #options: TabListOptions<T>;

  constructor(options: TabListOptions<T>) {
    super();
    this.#options = options;
    this.#actionBar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      ariaRole: "tablist",
      orientation: "horizontal",
      actionViewItemProvider: (action) => {
        if (!(action instanceof TabAction)) {
          throw new TypeError(`Unsupported TabList action: ${action.id}`);
        }
        return new TabActionViewItem(action, this.#options.onDelete);
      },
    }));
    this.#scrollable = this.own(new ScrollableElement({
      ownerDocument: options.ownerDocument,
      direction: "horizontal",
      horizontal: "auto",
      tabIndex: -1,
      wheel: { consume: "when-scrolling" },
    }));
    this.#scrollable.element.classList.add("zeta-tab-list");
    this.#scrollable.contentElement.classList.add(
      "zeta-tab-list-scroll-content",
    );
    this.#scrollable.append(this.#actionBar.element);
    this.element = this.#scrollable.element;
  }

  setTabs(
    tabs: readonly TabListItem<T>[],
    selectedId: string | undefined,
  ): void {
    const ids = new Set<string>();
    const tabIds = new Set<string>();
    for (const tab of tabs) {
      if (ids.has(tab.id)) {
        throw new TypeError(`Duplicate TabList item ID: ${tab.id}`);
      }
      if (tabIds.has(tab.tabId)) {
        throw new TypeError(`Duplicate TabList DOM ID: ${tab.tabId}`);
      }
      ids.add(tab.id);
      tabIds.add(tab.tabId);
    }
    if (selectedId !== undefined && !ids.has(selectedId)) {
      throw new RangeError(`Selected TabList item is not available: ${selectedId}`);
    }
    this.#actionBar.setActions(tabs.map((tab) => new TabAction(
      tab,
      tab.id === selectedId,
      this.#options.onActivate,
    )));
    if (selectedId !== undefined) this.#actionBar.setTabStop(selectedId);
    this.#scrollable.layout();
  }
}

class TabAction<T> implements IAction {
  readonly label: string;
  readonly tooltip: string;
  readonly enabled = true;

  constructor(
    readonly tab: TabListItem<T>,
    readonly checked: boolean,
    readonly activate: (value: T) => void,
  ) {
    this.label = tab.label;
    this.tooltip = tab.tooltip ?? tab.label;
  }

  get id(): string {
    return this.tab.id;
  }

  run(): void {
    this.activate(this.tab.value);
  }
}

class TabActionViewItem<T> extends ActionViewItem {
  readonly #tabAction: TabAction<T>;
  readonly #onDelete: ((value: T) => void) | undefined;
  #tab: HTMLButtonElement | undefined;

  constructor(action: TabAction<T>, onDelete: ((value: T) => void) | undefined) {
    super(action);
    this.#tabAction = action;
    this.#onDelete = onDelete;
  }

  override render(container: HTMLElement): void {
    if (this.#tab) {
      throw new Error(`TabList item is already rendered: ${this.action.id}`);
    }
    const item = this.#tabAction.tab;
    container.classList.add("zeta-tab");
    container.classList.toggle("active", this.#tabAction.checked);
    container.classList.toggle("icon", item.icon !== undefined);

    const tab = container.ownerDocument.createElement("button");
    this.#tab = tab;
    tab.id = item.tabId;
    tab.className = "zeta-tab-label";
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-selected", String(this.#tabAction.checked));
    tab.setAttribute("aria-label", item.ariaLabel ?? item.label);
    if (item.panelId) tab.setAttribute("aria-controls", item.panelId);
    tab.title = this.#tabAction.tooltip;
    const label = this.own(new IconLabel({
      label: item.label,
      icon: item.icon,
      ownerDocument: container.ownerDocument,
      title: this.#tabAction.tooltip,
    }));
    tab.append(label.element);
    container.append(tab);

    this.own(addDisposableListener(tab, "click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.#tabAction.run();
    }));
    if (this.#onDelete) {
      tab.setAttribute("aria-keyshortcuts", "Delete");
      this.own(addDisposableListener(tab, "keydown", (event) => {
        if (event.key !== "Delete") return;
        event.preventDefault();
        event.stopPropagation();
        this.#onDelete?.(item.value);
      }));
    }
    if (item.actions?.items.length) {
      const actions = this.own(new ActionBar({
        ownerDocument: container.ownerDocument,
        actions: item.actions.items,
        ariaLabel: item.actions.ariaLabel,
      }));
      actions.element.classList.add("zeta-tab-actions");
      container.append(actions.element);
    }
  }

  override focus(): void {
    this.#requireTab().focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.#requireTab().tabIndex = tabbable ? 0 : -1;
  }

  #requireTab(): HTMLButtonElement {
    if (!this.#tab) {
      throw new Error(`TabList item is not rendered: ${this.action.id}`);
    }
    return this.#tab;
  }
}
