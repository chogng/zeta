import { ToolBar } from "../../../base/browser/ui/toolbar/toolbar.js";
import { Separator } from "../../../base/common/actions.js";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import { createMenuEntryActionViewItem } from "./menuEntryActionViewItem.js";
import type { IMenuService } from "../common/menuService.js";
import { MenuId } from "../common/actions.js";

/** Keeps a ToolBar synchronized with one registered menu location. */
export class MenuWorkbenchToolBar extends ToolBar {
  constructor(
    menuService: IMenuService,
    contextMenuProvider: IContextMenuProvider,
    menuId: MenuId,
    ownerDocument: Document = document,
  ) {
    super({
      contextMenuProvider,
      ownerDocument,
      actionViewItemProvider: (action) => createMenuEntryActionViewItem(action, contextMenuProvider),
    });
    const menu = this.own(menuService.createMenu(menuId));
    const render = (): void => {
      const groups = menu.getActions();
      const primary = groups
        .filter(([group]) => group === "navigation")
        .flatMap(([, actions]) => actions);
      const secondary = Separator.join(
        ...groups
          .filter(([group]) => group !== "navigation")
          .map(([, actions]) => [...actions]),
      );
      this.setActions(primary, secondary);
    };
    this.own(menu.onDidChange(render));
    render();
  }
}
