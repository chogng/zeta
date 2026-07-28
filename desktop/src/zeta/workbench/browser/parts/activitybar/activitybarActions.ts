import { addDisposableListener } from "../../../../base/browser/dom.js";
import {
  ActionViewItem,
} from "../../../../base/browser/ui/actionbar/actionViewItems.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import type { IAction } from "../../../../base/common/actions.js";

/**
 * Activity Bar representation of a View Container action.
 *
 * The host ActionBar owns the item container. This view item owns its label,
 * interaction, focus, and selected presentation inside that container.
 */
export class ActivitybarActionViewItem extends ActionViewItem {
  #container: HTMLElement | undefined;

  constructor(action: IAction) {
    super(action);
  }

  override render(container: HTMLElement): void {
    if (this.#container) {
      throw new Error(
        `Activity Bar action view item is already rendered: ${this.action.id}`,
      );
    }
    this.#container = container;
    container.setAttribute("role", "tab");
    container.setAttribute(
      "aria-label",
      this.action.tooltip || this.action.label,
    );
    container.setAttribute(
      "aria-disabled",
      String(!this.action.enabled),
    );
    container.classList.toggle("icon", this.action.icon !== undefined);

    const label = container.ownerDocument.createElement("a");
    label.className = "zeta-action-label";
    label.tabIndex = -1;
    if (this.action.icon) appendIcon(this.action.icon, label);
    const labelText = container.ownerDocument.createElement("span");
    labelText.textContent = this.action.label;
    label.append(labelText);
    container.append(label);

    this.own(addDisposableListener(container, "click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.#run();
    }));
    this.own(addDisposableListener(
      container,
      "keydown",
      (event: KeyboardEvent) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        event.stopPropagation();
        this.#run();
      },
    ));
    this.setActive(this.action.checked === true);
  }

  setActive(active: boolean): void {
    const container = this.#requireContainer();
    container.setAttribute("aria-selected", String(active));
    container.tabIndex = active ? 0 : -1;
  }

  override focus(): void {
    this.#requireContainer().focus();
  }

  #run(): void {
    if (this.action.enabled) this.action.run();
  }

  #requireContainer(): HTMLElement {
    if (!this.#container) {
      throw new Error(
        `Activity Bar action view item is not rendered: ${this.action.id}`,
      );
    }
    return this.#container;
  }
}
