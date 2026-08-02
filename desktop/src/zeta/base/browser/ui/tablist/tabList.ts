import "./tablist.css";
import { addDisposableListener } from "../../dom.js";
import type { Icon } from "../../../common/icon.js";
import type { IAction } from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { ActionBar, type ActionBarOrientation, type ActionViewItemProvider } from "../actionbar/actionbar.js";
import { ScrollableElement } from "../scrollbar/scrollableElement.js";
import { TabAction, TabActionViewItem } from "./tabActionViewItem.js";

export { TAB_CLOSE_ACTION_ID } from "./tabActionViewItem.js";

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
  readonly state?: string;
  readonly tabId: string;
  readonly panelId?: string;
  readonly actions?: TabListActions;
}

/** Visual edge treatment for the ActionBar rendered inside a TabList. */
export type TabListPresentation = "flush" | "inset";

/** Construction inputs for a manually activated TabList. */
export interface TabListOptions<T> {
  readonly ownerDocument: Document;
  readonly ariaLabel: string;
  readonly presentation?: TabListPresentation;
  readonly orientation?: ActionBarOrientation;
  readonly onActivate: (value: T) => void;
  readonly onClose?: (value: T) => void;
  readonly closeActionIcon?: Icon;
  /** Non-tab actions retained after the selectable tabs. */
  readonly trailingActions?: readonly IAction[];
  readonly trailingActionViewItemProvider?: ActionViewItemProvider;
}

/**
 * Domain-neutral tab semantics built on the shared roving-focus engine.
 *
 * Arrow keys move focus without changing selection. Callers own content
 * activation and panel lifetimes, then provide the resulting selected ID.
 */
export class TabList<T> extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly actionBar: ActionBar;
  private readonly scrollable: ScrollableElement;
  private readonly activate: (value: T) => void;
  private readonly trailingActions: readonly IAction[];

  constructor(options: TabListOptions<T>) {
    super();
    this.activate = options.onActivate;
    const onClose = options.onClose;
    const closeActionIcon = options.closeActionIcon;
    this.trailingActions = options.trailingActions ?? [];
    const trailingActionIds = new Set(this.trailingActions.map((action) => action.id));
    const presentation = options.presentation ?? "flush";
    const orientation = options.orientation ?? "horizontal";
    this.actionBar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      ariaRole: "tablist",
      orientation,
      actionViewItemProvider: (action) => {
        if (!(action instanceof TabAction)) {
          if (!trailingActionIds.has(action.id)) {
            throw new TypeError(`Unsupported TabList action: ${action.id}`);
          }
          return options.trailingActionViewItemProvider?.(action);
        }
        return new TabActionViewItem(action, onClose, closeActionIcon);
      },
    }));
    const scrollableOptions = orientation === "vertical"
      ? { ownerDocument: options.ownerDocument, direction: "vertical" as const, vertical: "auto" as const, tabIndex: -1, wheel: { consume: "when-scrolling" as const } }
      : { ownerDocument: options.ownerDocument, direction: "horizontal" as const, horizontal: "auto" as const, tabIndex: -1, wheel: { consume: "when-scrolling" as const } };
    this.scrollable = this.own(new ScrollableElement(scrollableOptions));
    this.scrollable.element.classList.add("zeta-tab-list");
    this.scrollable.element.classList.add(`zeta-tab-list-${presentation}`);
    this.scrollable.contentElement.classList.add(
      "zeta-tab-list-scroll-content",
    );
    this.scrollable.append(this.actionBar.element);
    this.element = this.scrollable.element;
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
    for (const action of this.trailingActions) {
      if (ids.has(action.id)) {
        throw new TypeError(`TabList trailing action conflicts with tab ID: ${action.id}`);
      }
    }
    this.actionBar.setActions([
      ...tabs.map((tab) => new TabAction(
        tab,
        tab.id === selectedId,
        this.activate,
      )),
      ...this.trailingActions,
    ]);
    if (selectedId !== undefined) this.actionBar.setTabStop(selectedId);
    this.scrollable.layout();
  }
}
