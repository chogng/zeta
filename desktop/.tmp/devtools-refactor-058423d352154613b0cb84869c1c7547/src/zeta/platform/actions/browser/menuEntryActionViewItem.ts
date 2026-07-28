import {
  ActionViewItem,
  ButtonActionViewItem,
} from "../../../base/browser/ui/actionbar/actionViewItems.js";
import type {
  IContextMenuProvider,
} from "../../../base/browser/contextmenu.js";
import {
  DropdownMenuActionViewItem,
} from "../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import type { IAction } from "../../../base/common/actions.js";
import {
  MenuItemAction,
  SubmenuItemAction,
} from "../common/actions.js";

/**
 * ActionBar representation of one command resolved from a menu contribution.
 *
 * Popup-menu rows use base Menu's private view items instead; this platform
 * representation is for contributed actions rendered in toolbars and other
 * ActionBar hosts.
 */
export class MenuEntryActionViewItem extends ButtonActionViewItem {
  constructor(action: MenuItemAction) {
    super(action);
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-menu-entry");
  }
}

/**
 * ActionBar representation of a contributed submenu.
 *
 * The trigger retains toolbar semantics while its popup delegates all menu-row
 * rendering, keyboard navigation, and nested submenus to base Menu.
 */
export class SubmenuEntryActionViewItem
  extends DropdownMenuActionViewItem {
  constructor(
    action: SubmenuItemAction,
    contextMenuProvider: IContextMenuProvider,
  ) {
    super(action, () => action.actions, contextMenuProvider);
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-menu-entry");
  }
}

/**
 * Creates the ActionBar representation for a platform menu contribution.
 *
 * Returning undefined lets ActionBar use its base representation for actions
 * that were not produced by the platform menu service.
 */
export function createMenuEntryActionViewItem(
  action: IAction,
  contextMenuProvider: IContextMenuProvider,
): ActionViewItem | undefined {
  if (action instanceof MenuItemAction) {
    return new MenuEntryActionViewItem(action);
  }
  if (action instanceof SubmenuItemAction) {
    return new SubmenuEntryActionViewItem(action, contextMenuProvider);
  }
  return undefined;
}
