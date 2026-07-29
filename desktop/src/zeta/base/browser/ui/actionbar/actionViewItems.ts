import {
  Separator,
  type IAction,
} from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { Button } from "../button/button.js";

/**
 * Browser representation of one action inside an ActionBar.
 *
 * Implementations render into the container owned by their host and own the
 * resources they create for that representation.
 */
export abstract class ActionViewItem extends DisposableOwner {
  protected constructor(readonly action: IAction) {
    super();
  }

  abstract render(container: HTMLElement): void;

  /** Controls whether this item is the ActionBar's page-level Tab stop. */
  abstract setTabbable(tabbable: boolean): void;

  focus(): void {}
}

/** Default button representation for a runnable action. */
export class ButtonActionViewItem extends ActionViewItem {
  #button: Button | undefined;

  constructor(action: IAction) {
    super(action);
  }

  override render(container: HTMLElement): void {
    if (this.#button) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    this.#button = this.own(new Button({
      label: this.action.label,
      ownerDocument: container.ownerDocument,
      icon: this.action.icon,
      title: this.action.tooltip,
      enabled: this.action.enabled,
      checked: this.action.checked,
      onClick: () => this.runAction(),
    }));
    container.append(this.#button.element);
  }

  override focus(): void {
    this.button.element.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.button.element.tabIndex = tabbable ? 0 : -1;
  }

  protected get button(): Button {
    if (!this.#button) {
      throw new Error(`Action view item is not rendered: ${this.action.id}`);
    }
    return this.#button;
  }

  protected runAction(): unknown {
    return this.action.run();
  }
}

/** Non-interactive visual representation of a separator action. */
export class SeparatorActionViewItem extends ActionViewItem {
  #rendered = false;

  constructor(action: Separator) {
    super(action);
  }

  override render(container: HTMLElement): void {
    if (this.#rendered) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    this.#rendered = true;
    container.classList.add("zeta-action-view-item-separator");
    container.setAttribute("role", "separator");
  }

  override setTabbable(_tabbable: boolean): void {}
}

/** Creates the base representation used when a platform has no override. */
export function createActionViewItem(action: IAction): ActionViewItem {
  return action instanceof Separator
    ? new SeparatorActionViewItem(action)
    : new ButtonActionViewItem(action);
}
