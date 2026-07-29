import { addDisposableListener } from "../../dom.js";
import type { IAction } from "../../../common/actions.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../common/lifecycle.js";
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

/**
 * Owns action arrangement and composite-widget keyboard navigation.
 *
 * The optional ARIA role describes the rendered collection. Item providers
 * remain responsible for role-specific item state such as `aria-selected`.
 */
export class ActionBar extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #items = this.own(new ResettableDisposableGroup());
  readonly #entries: Array<{
    readonly action: IAction;
    readonly container: HTMLElement;
    readonly item: ActionViewItem;
  }> = [];
  readonly #actionViewItemProvider: ActionViewItemProvider | undefined;
  readonly #orientation: ActionBarOrientation;
  #tabStop: ActionViewItem | undefined;

  constructor(options: ActionBarOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-action-bar";
    element.classList.toggle("highlight-toggled", options.highlightToggledItems === true);
    element.setAttribute("role", options.ariaRole ?? "toolbar");
    this.#orientation = options.orientation ?? "horizontal";
    element.setAttribute("aria-orientation", this.#orientation);
    if (options.ariaLabel) {
      element.setAttribute("aria-label", options.ariaLabel);
    }
    this.#actionViewItemProvider = options.actionViewItemProvider;
    this.own(addDisposableListener(element, "keydown", (event) => {
      this.#handleNavigation(event);
    }));
    this.own(addDisposableListener(element, "focusin", () => {
      const activeElement = this.element.ownerDocument.activeElement;
      const entry = this.#entries.find(
        ({ container }) => container.contains(activeElement),
      );
      if (entry?.action.enabled) this.#setTabStop(entry.item);
    }));
    this.setActions(options.actions ?? []);
  }

  add(action: IAction): ActionViewItem {
    const container = this.element.ownerDocument.createElement("div");
    container.className = "zeta-action-view-item";
    container.classList.toggle("icon", action.icon !== undefined);
    container.dataset.actionId = action.id;
    container.setAttribute("role", "presentation");
    const item = this.#items.add(
      this.#actionViewItemProvider?.(action) ??
        createActionViewItem(action),
    );
    this.element.append(container);
    item.render(container);
    this.#entries.push({ action, container, item });
    if (action.enabled && !this.#tabStop) {
      this.#setTabStop(item);
    } else {
      item.setTabbable(false);
    }
    return item;
  }

  setActions(actions: readonly IAction[]): void {
    this.#items.clear();
    this.#entries.length = 0;
    this.#tabStop = undefined;
    this.element.replaceChildren();
    for (const action of actions) this.add(action);
  }

  /** Selects the action that participates in page-level Tab navigation. */
  setTabStop(actionId: string): void {
    const entry = this.#entries.find(
      ({ action }) => action.id === actionId && action.enabled,
    );
    if (!entry) {
      throw new RangeError(`Focusable ActionBar item not found: ${actionId}`);
    }
    this.#setTabStop(entry.item);
  }

  #handleNavigation(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.shiftKey
    ) {
      return;
    }
    const direction = this.#navigationDirection(event.key);
    if (direction === undefined) return;
    const entries = this.#entries.filter(({ action }) => action.enabled);
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
    this.#setTabStop(target);
    target.focus();
    event.preventDefault();
    event.stopPropagation();
  }

  #setTabStop(item: ActionViewItem): void {
    if (item === this.#tabStop) return;
    this.#tabStop?.setTabbable(false);
    this.#tabStop = item;
    item.setTabbable(true);
  }

  #navigationDirection(
    key: string,
  ): "first" | "last" | "next" | "previous" | undefined {
    if (key === "Home") return "first";
    if (key === "End") return "last";
    if (this.#orientation === "horizontal") {
      if (key === "ArrowRight") return "next";
      if (key === "ArrowLeft") return "previous";
      return undefined;
    }
    if (key === "ArrowDown") return "next";
    if (key === "ArrowUp") return "previous";
    return undefined;
  }
}
