import type { ActionBarOrientation, ActionViewItemProvider } from "../../../base/browser/ui/actionbar/actionbar.js";
import { ToolBar, type MoreActionsPlacement, type ToolBarPresentation } from "../../../base/browser/ui/toolbar/toolbar.js";
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
  readonly moreActionsPlacement?: MoreActionsPlacement;
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
      moreActionsPlacement: options.moreActionsPlacement,
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
  private readonly menuOptions: IMenuActionOptions | undefined;
  private readonly menu: IMenu & Disposable;
  private supplementalSecondaryActions: readonly IAction[] = [];

  constructor(
    menuService: IMenuService,
    contextMenuProvider: IContextMenuProvider,
    menuId: MenuId,
    ownerDocument: Document = document,
    options: MenuWorkbenchToolBarOptions = {},
  ) {
    super(contextMenuProvider, ownerDocument, options);
    this.menuOptions = options.menuOptions;
    const menu = this.own(menuService.createMenu(menuId));
    this.menu = menu;
    this.own(menu.onDidChange(() => this.update()));
    this.update();
  }

  refresh(): void {
    this.update();
  }

  /** Adds host-owned overflow actions after this menu's secondary groups. */
  setSupplementalSecondaryActions(actions: readonly IAction[]): void {
    this.supplementalSecondaryActions = actions;
    this.update();
  }

  override setActions(_primaryActions: readonly IAction[], _secondaryActions: readonly IAction[] = []): never {
    throw new Error("MenuWorkbenchToolBar actions are owned by its MenuId");
  }

  private update(): void {
    const groups = this.menu.getActions(this.menuOptions);
    const primary = groups
      .filter(([group]) => group === "navigation")
      .flatMap(([, actions]) => actions);
    const menuSecondary = Separator.join(
      ...groups
        .filter(([group]) => group !== "navigation")
        .map(([, actions]) => [...actions]),
    );
    const secondary = Separator.join(
      menuSecondary,
      [...this.supplementalSecondaryActions],
    );
    super.setActions(primary, secondary);
  }
}
