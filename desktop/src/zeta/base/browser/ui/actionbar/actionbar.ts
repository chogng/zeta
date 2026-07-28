import type { IAction } from "../../../common/actions.js";
import {
  DisposableOwner,
  ResettableDisposableGroup,
} from "../../../common/lifecycle.js";
import {
  type ActionViewItem,
  createActionViewItem,
} from "./actionViewItems.js";

export type ActionViewItemProvider = (
  action: IAction,
) => ActionViewItem | undefined;

export interface ActionBarOptions {
  readonly ownerDocument?: Document;
  readonly actions?: readonly IAction[];
  readonly actionViewItemProvider?: ActionViewItemProvider;
  readonly ariaRole?: "toolbar" | "tablist";
  readonly ariaLabel?: string;
}

/** Owns and arranges action view items without interpreting action subtypes. */
export class ActionBar extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #items = this.own(new ResettableDisposableGroup());
  readonly #actionViewItemProvider: ActionViewItemProvider | undefined;

  constructor(options: ActionBarOptions = {}) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-action-bar";
    element.setAttribute("role", options.ariaRole ?? "toolbar");
    if (options.ariaLabel) {
      element.setAttribute("aria-label", options.ariaLabel);
    }
    this.#actionViewItemProvider = options.actionViewItemProvider;
    this.setActions(options.actions ?? []);
  }

  add(action: IAction): ActionViewItem {
    const container = this.element.ownerDocument.createElement("div");
    container.className = "zeta-action-view-item";
    container.dataset.actionId = action.id;
    container.setAttribute("role", "presentation");
    const item = this.#items.add(
      this.#actionViewItemProvider?.(action) ??
        createActionViewItem(action),
    );
    this.element.append(container);
    item.render(container);
    return item;
  }

  setActions(actions: readonly IAction[]): void {
    this.#items.clear();
    this.element.replaceChildren();
    for (const action of actions) this.add(action);
  }
}
