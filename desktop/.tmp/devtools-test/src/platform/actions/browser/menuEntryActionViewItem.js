import { ButtonActionViewItem, } from "../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem, } from "../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import { MenuItemAction, SubmenuItemAction, } from "../common/actions.js";
/**
 * ActionBar representation of one command resolved from a menu contribution.
 *
 * Popup-menu rows use base Menu's private view items instead; this platform
 * representation is for contributed actions rendered in toolbars and other
 * ActionBar hosts.
 */
export class MenuEntryActionViewItem extends ButtonActionViewItem {
    constructor(action) {
        super(action);
    }
    render(container) {
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
export class SubmenuEntryActionViewItem extends DropdownMenuActionViewItem {
    constructor(action, contextMenuProvider) {
        super(action, () => action.actions, contextMenuProvider);
    }
    render(container) {
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
export function createMenuEntryActionViewItem(action, contextMenuProvider) {
    if (action instanceof MenuItemAction) {
        return new MenuEntryActionViewItem(action);
    }
    if (action instanceof SubmenuItemAction) {
        return new SubmenuEntryActionViewItem(action, contextMenuProvider);
    }
    return undefined;
}
