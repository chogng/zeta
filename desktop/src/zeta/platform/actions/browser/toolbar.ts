import type { ActionBarOrientation, ActionViewItemProvider } from "../../../base/browser/ui/actionbar/actionbar.js";
import { ToolBar, type ToolBarPresentation } from "../../../base/browser/ui/toolbar/toolbar.js";
import { Separator, type IAction } from "../../../base/common/actions.js";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import { createMenuEntryActionViewItem } from "./menuEntryActionViewItem.js";
import { MenuId, type IMenuActionOptions } from "../common/actions.js";
import type { IMenu, IMenuService } from "../common/menuService.js";

export interface WorkbenchToolBarOptions {
  readonly ariaLabel?: string;
  readonly orientation?: ActionBarOrientation;
  readonly actionViewItemProvider?: ActionViewItemProvider;
  readonly presentation?: ToolBarPresentation;
  readonly highlightToggledItems?: boolean;
}

/**
 * Adapts platform action representations to the base ToolBar.
 *
 * Callers still own the primary and secondary action lists. Menu-backed
 * population belongs to MenuWorkbenchToolBar.
 */
export class WorkbenchToolBar extends ToolBar {
  constructor(
    contextMenuProvider: IContextMenuProvider,
    ownerDocument: Document = document,
    options: WorkbenchToolBarOptions = {},
  ) {
    super({
      contextMenuProvider,
      ownerDocument,
      ariaLabel: options.ariaLabel,
      orientation: options.orientation,
      presentation: options.presentation,
      highlightToggledItems: options.highlightToggledItems,
      actionViewItemProvider: (action) =>
        options.actionViewItemProvider?.(action) ??
        createMenuEntryActionViewItem(action, contextMenuProvider),
    });
  }
}

export interface MenuWorkbenchToolBarOptions extends WorkbenchToolBarOptions {
  readonly menuOptions?: IMenuActionOptions;
}

/** Keeps a WorkbenchToolBar synchronized with one registered menu location. */
export class MenuWorkbenchToolBar extends WorkbenchToolBar {
  readonly #menuOptions: IMenuActionOptions | undefined;
  readonly #menu: IMenu & Disposable;

  constructor(
    menuService: IMenuService,
    contextMenuProvider: IContextMenuProvider,
    menuId: MenuId,
    ownerDocument: Document = document,
    options: MenuWorkbenchToolBarOptions = {},
  ) {
    super(contextMenuProvider, ownerDocument, options);
    this.#menuOptions = options.menuOptions;
    const menu = this.own(menuService.createMenu(menuId));
    this.#menu = menu;
    this.own(menu.onDidChange(() => this.#update()));
    this.#update();
  }

  refresh(): void {
    this.#update();
  }

  override setActions(_primaryActions: readonly IAction[], _secondaryActions: readonly IAction[] = []): never {
    throw new Error("MenuWorkbenchToolBar actions are owned by its MenuId");
  }

  #update(): void {
    const groups = this.#menu.getActions(this.#menuOptions);
    const primary = groups
      .filter(([group]) => group === "navigation")
      .flatMap(([, actions]) => actions);
    const secondary = Separator.join(
      ...groups
        .filter(([group]) => group !== "navigation")
        .map(([, actions]) => [...actions]),
    );
    super.setActions(primary, secondary);
  }
}
