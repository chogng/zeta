import { addDisposableListener } from "../../dom.js";
import type { IAction } from "../../../common/actions.js";
import { DisposableOwner, DisposableStore } from "../../../common/lifecycle.js";
import { type ActionViewItem, createActionViewItem } from "./actionViewItems.js";

export type ActionBarOrientation = "horizontal" | "vertical";

export type ActionViewItemProvider = (
  action: IAction,
) => ActionViewItem | undefined;

export interface ActionBarOptions {
  readonly ownerDocument?: Document;
  readonly actions?: readonly IAction[];
  readonly ariaLabel?: string;
  readonly ariaRole?: "toolbar" | "tablist";
  readonly orientation?: ActionBarOrientation;
  readonly actionViewItemProvider?: ActionViewItemProvider;
  readonly highlightToggledItems?: boolean;
}

interface ActionBarEntry {
  action: IAction;
  readonly container: HTMLElement;
  item: ActionViewItem;
  store: DisposableStore;
}

/**
 * Owns action arrangement and composite-widget keyboard navigation.
 *
 * The optional ARIA role describes the rendered collection. Item providers
 * remain responsible for role-specific item state such as `aria-selected`.
 */
export class ActionBar extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly entries: ActionBarEntry[] = [];
  private readonly actionViewItemProvider: ActionViewItemProvider | undefined;
  private readonly orientation: ActionBarOrientation;
  private tabStop: ActionViewItem | undefined;

  constructor(options: ActionBarOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-action-bar";
    this.orientation = options.orientation ?? "horizontal";
    element.classList.toggle("vertical", this.orientation === "vertical");
    element.classList.toggle("highlight-toggled", options.highlightToggledItems === true);
    element.setAttribute("role", options.ariaRole ?? "toolbar");
    element.setAttribute("aria-orientation", this.orientation);
    if (options.ariaLabel) {
      element.setAttribute("aria-label", options.ariaLabel);
    }
    this.actionViewItemProvider = options.actionViewItemProvider;
    this.own(addDisposableListener(element, "keydown", (event) => {
      this.handleNavigation(event);
    }));
    this.own(addDisposableListener(element, "focusin", () => {
      const activeElement = this.element.ownerDocument.activeElement;
      const entry = this.entries.find(
        ({ container }) => container.contains(activeElement),
      );
      if (entry?.action.enabled) this._setTabStop(entry.item);
    }));
    this.defer(() => this.clearActions());
    this.setActions(options.actions ?? []);
  }

  add(action: IAction): ActionViewItem {
    const container = this.element.ownerDocument.createElement("div");
    container.className = "zeta-action-view-item";
    container.classList.toggle("icon", action.icon !== undefined);
    container.dataset.actionId = action.id;
    container.setAttribute("role", "presentation");
    const entry = this.createEntry(action, container);
    this.element.append(container);
    this.entries.push(entry);
    if (action.enabled && !this.tabStop) {
      this._setTabStop(entry.item);
    } else {
      entry.item.setTabbable(false);
    }
    return entry.item;
  }

  setActions(actions: readonly IAction[]): void {
    this.clearActions();
    this.tabStop = undefined;
    this.element.replaceChildren();
    for (const action of actions) this.add(action);
  }

  /** Updates retained action slots when menu structure and ordering are stable. */
  updateActions(actions: readonly IAction[]): void {
    if (!this.hasMatchingStructure(actions)) {
      this.setActions(actions);
      return;
    }
    for (let index = 0; index < actions.length; index += 1) {
      this.replaceEntry(this.entries[index]!, actions[index]!);
    }
  }

  /** Selects the action that participates in page-level Tab navigation. */
  setTabStop(actionId: string): void {
    const entry = this.entries.find(
      ({ action }) => action.id === actionId && action.enabled,
    );
    if (!entry) {
      throw new RangeError(`Focusable ActionBar item not found: ${actionId}`);
    }
    this._setTabStop(entry.item);
  }

  private handleNavigation(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey
    ) {
      return;
    }
    const direction = this.navigationDirection(event.key);
    if (direction === undefined) return;
    const entries = this.entries.filter(({ action }) => action.enabled);
    if (entries.length === 0) return;
    const activeElement = this.element.ownerDocument.activeElement;
    const currentIndex = entries.findIndex(
      ({ container }) => container.contains(activeElement),
    );
    let targetIndex: number;
    if (direction === "first") {
      targetIndex = 0;
    } else if (direction === "last") {
      targetIndex = entries.length - 1;
    } else if (currentIndex === -1) {
      targetIndex = direction === "next" ? 0 : entries.length - 1;
    } else {
      const delta = direction === "next" ? 1 : -1;
      targetIndex = (currentIndex + delta + entries.length) % entries.length;
    }
    const target = entries[targetIndex]!.item;
    this._setTabStop(target);
    target.focus();
    event.preventDefault();
    event.stopPropagation();
  }

  private _setTabStop(item: ActionViewItem): void {
    if (item === this.tabStop) return;
    this.tabStop?.setTabbable(false);
    this.tabStop = item;
    item.setTabbable(true);
  }

  private clearActions(): void {
    for (const entry of this.entries) entry.store.dispose();
    this.entries.length = 0;
  }

  private createEntry(action: IAction, container: HTMLElement): ActionBarEntry {
    const store = new DisposableStore();
    const item = store.add(
      this.actionViewItemProvider?.(action) ??
        createActionViewItem(action),
    );
    item.render(container);
    return { action, container, item, store };
  }

  private hasMatchingStructure(actions: readonly IAction[]): boolean {
    return actions.length === this.entries.length && actions.every(
      (action, index) => action.id === this.entries[index]?.action.id,
    );
  }

  private replaceEntry(entry: ActionBarEntry, action: IAction): void {
    const wasTabStop = entry.item === this.tabStop;
    entry.store.dispose();
    entry.container.replaceChildren();
    entry.container.classList.toggle("icon", action.icon !== undefined);
    entry.action = action;
    const replacement = this.createEntry(action, entry.container);
    entry.item = replacement.item;
    entry.store = replacement.store;
    if (wasTabStop) this.tabStop = entry.item;
    entry.item.setTabbable(wasTabStop);
  }

  private navigationDirection(
    key: string,
  ): "first" | "last" | "next" | "previous" | undefined {
    if (key === "Home") return "first";
    if (key === "End") return "last";
    if (this.orientation === "horizontal") {
      if (key === "ArrowRight") return "next";
      if (key === "ArrowLeft") return "previous";
      return undefined;
    }
    if (key === "ArrowDown") return "next";
    if (key === "ArrowUp") return "previous";
    return undefined;
  }
}
