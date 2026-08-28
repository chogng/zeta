import type { ActionBarOrientation, ActionViewItemProvider } from "../../../base/browser/ui/actionbar/actionbar.js";
import type { AnchorPosition } from "../../../base/browser/ui/contextview/contextview.js";
import { ToolBar, type MoreActionsPlacement, type ToolBarPresentation } from "../../../base/browser/ui/toolbar/toolbar.js";
import { Separator, type IAction } from "../../../base/common/actions.js";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import type { IContextKeyService } from "../../contextkey/common/contextkey.js";
import { createMenuEntryActionViewItem } from "./menuEntryActionViewItem.js";
import { MenuId, type IMenuActionOptions } from "../common/actions.js";
import type { IMenu, IMenuChangeEvent, IMenuService } from "../common/menuService.js";

export interface WorkbenchToolBarOptions {
	readonly ariaLabel?: string;
	readonly orientation?: ActionBarOrientation;
	readonly actionViewItemProvider?: ActionViewItemProvider;
	readonly presentation?: ToolBarPresentation;
	readonly highlightToggledItems?: boolean;
	readonly moreActionsPlacement?: MoreActionsPlacement;
	readonly hoverAnchorPosition?: AnchorPosition;
}

/**
 * Adapts platform action representations to the base ToolBar.
 *
 * Callers still own the primary and secondary action lists. Menu-backed
 * population belongs to MenuWorkbenchToolBar.
 */
export class WorkbenchToolBar extends ToolBar {
	constructor(
		container: HTMLElement,
		contextMenuProvider: IContextMenuProvider,
		options: WorkbenchToolBarOptions = {},
	) {
		super(container, {
			contextMenuProvider,
			ariaLabel: options.ariaLabel,
			orientation: options.orientation,
			presentation: options.presentation,
			highlightToggledItems: options.highlightToggledItems,
			moreActionsPlacement: options.moreActionsPlacement,
			hoverAnchorPosition: options.hoverAnchorPosition,
			actionViewItemProvider: (action, actionViewItemOptions) =>
				options.actionViewItemProvider?.(action, actionViewItemOptions) ??
				createMenuEntryActionViewItem(action, contextMenuProvider, actionViewItemOptions),
		});
	}
}

export interface MenuWorkbenchToolBarOptions extends WorkbenchToolBarOptions {
	readonly menuOptions?: IMenuActionOptions;
	readonly contextKeyService?: IContextKeyService;
}

/** Keeps a WorkbenchToolBar synchronized with one registered menu location. */
export class MenuWorkbenchToolBar extends WorkbenchToolBar {
	private readonly menuOptions: IMenuActionOptions | undefined;
	private readonly menu: IMenu;

	constructor(
		container: HTMLElement,
		menuService: IMenuService,
		contextMenuProvider: IContextMenuProvider,
		menuId: MenuId,
		options: MenuWorkbenchToolBarOptions = {},
	) {
		super(container, contextMenuProvider, options);
		this.menuOptions = options.menuOptions;
		const menu = this._register(menuService.createMenu(menuId, options.contextKeyService));
		this.menu = menu;
		this._register(menu.onDidChange((event) => this.update(event)));
		this.update();
	}

	refresh(): void {
		this.update();
	}

	override setActions(_primaryActions: readonly IAction[], _secondaryActions: readonly IAction[] = []): never {
		throw new Error("MenuWorkbenchToolBar actions are owned by its MenuId");
	}

	private update(event?: IMenuChangeEvent): void {
		const groups = this.menu.getActions(this.menuOptions);
		const primary = groups
			.filter(([group]) => group === "navigation")
			.flatMap(([, actions]) => actions);
		const menuSecondary = Separator.join(
			...groups
				.filter(([group]) => group !== "navigation")
				.map(([, actions]) => [...actions]),
		);
		const empty = primary.length === 0 && menuSecondary.length === 0;
		this.element.hidden = empty;
		this.element.classList.toggle("empty", empty);
		if (event?.isStructuralChange === false) {
			super.updateActions(primary, menuSecondary);
			return;
		}
		super.setActions(primary, menuSecondary);
	}
}
