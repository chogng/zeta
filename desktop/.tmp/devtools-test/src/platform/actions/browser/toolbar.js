import { ActionBar } from "../../../base/browser/ui/actionbar/actionbar.js";
import { createMenuEntryActionViewItem, } from "./menuEntryActionViewItem.js";
import { Separator } from "../../../base/common/actions.js";
/** Keeps an ActionBar synchronized with one registered menu location. */
export class MenuWorkbenchToolBar extends ActionBar {
    constructor(menuService, contextMenuProvider, menuId, ownerDocument = document) {
        super({
            ownerDocument,
            actionViewItemProvider: (action) => createMenuEntryActionViewItem(action, contextMenuProvider),
        });
        const menu = this.own(menuService.createMenu(menuId));
        const render = () => {
            const actions = Separator.join(...menu.getActions()
                .map(([, groupActions]) => [...groupActions]));
            this.setActions(actions);
        };
        this.own(menu.onDidChange(render));
        render();
    }
}
