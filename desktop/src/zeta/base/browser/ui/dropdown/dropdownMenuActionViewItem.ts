import type {
  IActionContextMenuOptions,
  IContextMenuProvider,
} from "../../contextmenu.js";
import { addDisposableListener, stopEvent } from "../../dom.js";
import type { IAction } from "../../../common/actions.js";
import { Emitter } from "../../../common/event.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";
import { ActionViewItem } from "../actionbar/actionViewItems.js";
import { Button } from "../button/button.js";
import { appendIcon } from "../icon/icon.js";

export type DropdownMenuActions =
  | readonly IAction[]
  | (() => readonly IAction[]);

/**
 * ActionBar item that delegates a dropdown action menu to the host provider.
 *
 * The provider owns menu rendering, positioning, native/browser selection, and
 * dismissal. This item owns only its trigger and expanded accessibility state.
 */
export class DropdownMenuActionViewItem extends ActionViewItem {
  private readonly actions: DropdownMenuActions;
  private readonly contextMenuProvider: IContextMenuProvider;
  private readonly _onDidChangeVisibility = this.own(new Emitter<boolean>());
  readonly onDidChangeVisibility = this._onDidChangeVisibility.event;
  private button: Button | undefined;
  private visible = false;

  constructor(
    action: IAction,
    actions: DropdownMenuActions,
    contextMenuProvider: IContextMenuProvider,
  ) {
    super(action);
    this.actions = actions;
    this.contextMenuProvider = contextMenuProvider;
  }

  override render(container: HTMLElement): void {
    if (this.button) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    const button = this.own(new Button({
      label: this.action.label,
      ownerDocument: container.ownerDocument,
      icon: this.action.icon,
      title: this.action.tooltip,
      enabled: this.action.enabled,
      onClick: () => this.show(),
    }));
    this.button = button;
    container.classList.add("zeta-dropdown-menu-action-view-item");
    button.element.setAttribute("aria-haspopup", "menu");
    button.element.setAttribute("aria-expanded", "false");
    const indicator = container.ownerDocument.createElement("span");
    indicator.className = "zeta-dropdown-menu-indicator";
    appendIcon(lxiconsLibrary.dropdownIndicator, indicator);
    button.element.append(indicator);
    container.append(button.element);
    this.own(addDisposableListener(button.element, "keydown", (event) => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      stopEvent(event);
      this.show();
    }));
  }

  override focus(): void {
    this.button?.element.focus();
  }

  override setTabbable(tabbable: boolean): void {
    if (this.button) this.button.element.tabIndex = tabbable ? 0 : -1;
  }

  show(): void {
    const button = this.button;
    if (!button?.enabled || this.visible) return;
    const actions = typeof this.actions === "function"
      ? this.actions()
      : this.actions;
    if (actions.length === 0) return;

    this.setVisible(true);
    const options: IActionContextMenuOptions = {
      anchor: button.element,
      actions,
      onHide: () => this.setVisible(false),
    };
    try {
      this.contextMenuProvider.showContextMenu(options);
    } catch (error) {
      this.setVisible(false);
      throw error;
    }
  }

  private setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    this.button?.element.setAttribute("aria-expanded", String(visible));
    this._onDidChangeVisibility.fire(visible);
  }
}
