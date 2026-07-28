import { ActionBar } from "../../../base/browser/ui/actionbar/actionbar.js";
import { Separator } from "../../../base/common/actions.js";
import {
  createMenuEntryActionViewItem,
} from "./menuEntryActionViewItem.js";
import {
  type IMenuService,
} from "../common/menuService.js";
import { MenuId } from "../common/actions.js";

/** Keeps an ActionBar synchronized with one registered action location. */
export class WorkbenchToolBar extends ActionBar {
  constructor(
    menuService: IMenuService,
    menuId: MenuId,
    ownerDocument: Document = document,
  ) {
    super({
      ownerDocument,
      actionViewItemProvider: (action) =>
        createMenuEntryActionViewItem(action),
    });
    const menu = this.own(menuService.createMenu(menuId));
    const render = (): void => {
      const actions = Separator.join(
        ...menu.getActions()
          .map(([, groupActions]) => [...groupActions]),
      );
      this.setActions(actions);
    };
    this.own(menu.onDidChange(render));
    render();
  }
}
