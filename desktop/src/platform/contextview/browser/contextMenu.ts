import {
  type IAction,
  Separator,
} from "../../../base/common/actions.js";
import type { Event } from "../../../base/common/event.js";
import {
  type IMenuActionOptions,
  MenuId,
} from "../../actions/common/actions.js";
import type {
  IMenuService,
} from "../../actions/common/menuService.js";
import {
  createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

export interface IContextMenuPoint {
  readonly x: number;
  readonly y: number;
}

export type ContextMenuAnchor = Element | IContextMenuPoint;

interface IBaseContextMenuOptions {
  readonly anchor: ContextMenuAnchor;
  readonly onHide?: (didCancel: boolean) => void;
}

export interface IActionContextMenuOptions
  extends IBaseContextMenuOptions {
  readonly actions: readonly IAction[];
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
export interface IContextMenuService {
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
  if ("actions" in options) return trimSeparators(options.actions);
  const actions = menuService
    .getMenuActions(options.menuId, options.menuActionOptions)
    .flatMap(([, groupActions], index, groups) => [
      ...groupActions,
      ...(index < groups.length - 1 ? [new Separator()] : []),
    ]);
  return trimSeparators(actions);
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
