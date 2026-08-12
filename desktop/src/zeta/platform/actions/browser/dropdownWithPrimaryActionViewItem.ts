import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { ActionViewItem, ButtonActionViewItem, type ActionViewItemOptions } from "../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem, type DropdownMenuActions } from "../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import type { IAction } from "../../../base/common/actions.js";
import { assertDefined } from "../../../base/common/types.js";

/**
 * Presents one primary action and its related menu as a single split action.
 *
 * The primary trigger runs the supplied action. The compact dropdown trigger
 * opens the related menu, and left/right navigation moves between both parts
 * before the containing ActionBar resumes navigation between items.
 */
export class DropdownWithPrimaryActionViewItem extends ActionViewItem {
  private readonly primaryItem: ButtonActionViewItem;
  private readonly dropdownItem: DropdownMenuActionViewItem;
  private primaryButton: HTMLButtonElement | undefined;
  private dropdownButton: HTMLButtonElement | undefined;

  constructor(
    primaryAction: IAction,
    dropdownAction: IAction,
    dropdownActions: DropdownMenuActions,
    contextMenuProvider: IContextMenuProvider,
    options: ActionViewItemOptions = {},
  ) {
    super(primaryAction, options);
    this.primaryItem = this.own(new ButtonActionViewItem(primaryAction, options));
    this.dropdownItem = this.own(new DropdownMenuActionViewItem(dropdownAction, dropdownActions, contextMenuProvider, options));
  }

  override render(container: HTMLElement): void {
    if (this.primaryButton || this.dropdownButton) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    container.classList.add("zeta-dropdown-with-primary-action-view-item");
    container.classList.toggle("disabled", !this.action.enabled);

    const primaryContainer = container.ownerDocument.createElement("div");
    primaryContainer.className = "zeta-dropdown-with-primary-primary";
    primaryContainer.classList.toggle("icon", this.action.icon !== undefined);
    this.primaryItem.render(primaryContainer);

    const dropdownContainer = container.ownerDocument.createElement("div");
    dropdownContainer.className = "zeta-dropdown-with-primary-dropdown";
    this.dropdownItem.render(dropdownContainer);

    const primaryButton = primaryContainer.querySelector<HTMLButtonElement>("button");
    const dropdownButton = dropdownContainer.querySelector<HTMLButtonElement>("button");
    assertDefined(primaryButton, `Primary action button was not rendered: ${this.action.id}`);
    assertDefined(dropdownButton, `Dropdown action button was not rendered: ${this.action.id}`);
    this.primaryButton = primaryButton;
    this.dropdownButton = dropdownButton;
    dropdownButton.setAttribute("aria-label", dropdownActionLabel(this.dropdownItem.action));
    container.append(primaryContainer, dropdownContainer);
    this.own(this.dropdownItem.onDidChangeVisibility((visible) => {
      container.classList.toggle("active", visible);
    }));

    this.own(addDisposableListener(primaryButton, "keydown", (event) => {
      if (event.key !== "ArrowRight" || dropdownButton.disabled) return;
      stopEvent(event);
      primaryButton.tabIndex = -1;
      dropdownButton.tabIndex = 0;
      dropdownButton.focus();
    }));
    this.own(addDisposableListener(dropdownButton, "keydown", (event) => {
      if (event.key !== "ArrowLeft" || primaryButton.disabled) return;
      stopEvent(event);
      dropdownButton.tabIndex = -1;
      primaryButton.tabIndex = 0;
      primaryButton.focus();
    }));
  }

  override focus(): void {
    this.primaryButton?.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.primaryItem.setTabbable(tabbable);
    this.dropdownItem.setTabbable(false);
  }
}

function dropdownActionLabel(action: IAction): string {
  return action.label || action.tooltip;
}
