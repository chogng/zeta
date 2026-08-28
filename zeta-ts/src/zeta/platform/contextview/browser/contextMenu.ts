import {
	type IAction,
	Separator,
} from "../../../base/common/actions.js";
import type { Event } from "../../../base/common/event.js";
import {
	type ContextMenuAnchor,
	type IActionContextMenuOptions,
	type IContextMenuProvider,
	type IContextMenuPoint,
} from "../../../base/browser/contextmenu.js";
import {
	type IMenuActionOptions,
	MenuId,
} from "../../actions/common/actions.js";
import type {
	IMenuService,
} from "../../actions/common/menuService.js";
import { getFlatContextMenuActions, resolveAlternativeMenuActions, shouldUseAlternativeMenuActions } from "../../actions/browser/menuEntryActionViewItem.js";
import {
	createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

export type {
	ContextMenuAnchor,
	IActionContextMenuOptions,
	IContextMenuPoint,
};

interface IBaseContextMenuOptions {
	readonly anchor: ContextMenuAnchor;
	readonly onHide?: (didCancel: boolean) => void;
}

export interface IMenuContextMenuOptions
	extends IBaseContextMenuOptions {
	readonly menuId: MenuId;
	readonly menuActionOptions?: IMenuActionOptions;
}

export type ContextMenuOptions =
	| IActionContextMenuOptions
	| IMenuContextMenuOptions;

/** Presents action menus using the current host's fixed rendering policy. */
export interface IContextMenuService extends IContextMenuProvider {
	readonly onDidShowContextMenu: Event<void>;
	readonly onDidHideContextMenu: Event<void>;

	showContextMenu(options: ContextMenuOptions): void;
	hideContextMenu(): void;
}

export const IContextMenuService =
	createServiceIdentifier<IContextMenuService>("contextMenuService");

export function resolveContextMenuActions(
	options: ContextMenuOptions,
	menuService: IMenuService,
): readonly IAction[] {
	const targetWindow = "ownerDocument" in options.anchor
		? options.anchor.ownerDocument.defaultView
		: window;
	if ("actions" in options) {
		const actions = targetWindow
			? resolveAlternativeMenuActions(options.actions, shouldUseAlternativeMenuActions(targetWindow))
			: options.actions;
		return trimSeparators(actions);
	}
	return trimSeparators(getFlatContextMenuActions(
		menuService.getMenuActions(options.menuId, options.menuActionOptions),
		undefined,
		targetWindow ?? undefined,
	));
}

function trimSeparators(actions: readonly IAction[]): readonly IAction[] {
	const result: IAction[] = [];
	for (const action of actions) {
		if (
			action instanceof Separator &&
			(result.length === 0 || result[result.length - 1] instanceof Separator)
		) {
			continue;
		}
		result.push(action);
	}
	if (result[result.length - 1] instanceof Separator) result.pop();
	return result;
}
