import {
  Separator,
  type IAction,
} from "../../../base/common/actions.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type {
  ICommandService,
} from "../../commands/common/command-registry.js";
import type {
  IContextKeyService,
} from "../../contextkey/common/contextkey.js";
import {
  type IMenuActionOptions,
  type IMenuRegistryChangeEvent,
  isMenuItem,
  MenuId,
  MenuItemAction,
  MenusRegistry,
  type MenuRegistryItem,
  SubmenuItemAction,
} from "./actions.js";

export type MenuActionGroup = readonly [
  group: string,
  actions: readonly IAction[],
];

export interface IMenu {
  readonly onDidChange: Event<void>;

  getActions(options?: IMenuActionOptions): readonly MenuActionGroup[];
}

export interface IMenuService {
  createMenu(id: MenuId): IMenu & Disposable;

  getMenuActions(
    id: MenuId,
    options?: IMenuActionOptions,
  ): readonly MenuActionGroup[];
}

/** Resolves registered menu contributions for one workbench context. */
export class MenuService implements IMenuService {
  readonly #commandService: ICommandService;
  readonly #contextKeyService: IContextKeyService;

  constructor(
    commandService: ICommandService,
    contextKeyService: IContextKeyService,
  ) {
    this.#commandService = commandService;
    this.#contextKeyService = contextKeyService;
  }

  createMenu(id: MenuId): IMenu & Disposable {
    return new Menu(id, this.#commandService, this.#contextKeyService);
  }

  getMenuActions(
    id: MenuId,
    options?: IMenuActionOptions,
  ): readonly MenuActionGroup[] {
    return resolveMenu(
      id,
      this.#commandService,
      this.#contextKeyService,
      options,
      new Set(),
    );
  }
}

class Menu extends DisposableOwner implements IMenu {
  readonly #onDidChange = this.own(new Emitter<void>());
  readonly onDidChange = this.#onDidChange.event;
  readonly #id: MenuId;
  readonly #commandService: ICommandService;
  readonly #contextKeyService: IContextKeyService;

  constructor(
    id: MenuId,
    commandService: ICommandService,
    contextKeyService: IContextKeyService,
  ) {
    super();
    this.#id = id;
    this.#commandService = commandService;
    this.#contextKeyService = contextKeyService;
    this.own(MenusRegistry.onDidChangeMenu(
      (_event: IMenuRegistryChangeEvent) => {
        this.#onDidChange.fire();
      },
    ));
    this.own(this.#contextKeyService.onDidChangeContext(() => {
      this.#onDidChange.fire();
    }));
  }

  getActions(options?: IMenuActionOptions): readonly MenuActionGroup[] {
    return resolveMenu(
      this.#id,
      this.#commandService,
      this.#contextKeyService,
      options,
      new Set(),
    );
  }
}

function resolveMenu(
  id: MenuId,
  commandService: ICommandService,
  contextKeyService: IContextKeyService,
  options: IMenuActionOptions | undefined,
  ancestors: Set<MenuId>,
): readonly MenuActionGroup[] {
  if (ancestors.has(id)) {
    throw new Error(`Menu contribution cycle detected at '${id.id}'`);
  }

  const nextAncestors = new Set(ancestors);
  nextAncestors.add(id);
  const sorted = [...MenusRegistry.getMenuItems(id)].sort(compareMenuItems);
  const groups = new Map<string, IAction[]>();

  for (const item of sorted) {
    if (!contextKeyService.contextMatchesRules(item.when)) continue;
    const action = resolveItem(
      item,
      commandService,
      contextKeyService,
      options,
      nextAncestors,
    );
    if (!action) continue;
    const group = item.group ?? "";
    const actions = groups.get(group);
    if (actions) actions.push(action);
    else groups.set(group, [action]);
  }

  return [...groups].map(([group, actions]) => [group, actions] as const);
}

function resolveItem(
  item: MenuRegistryItem,
  commandService: ICommandService,
  contextKeyService: IContextKeyService,
  options: IMenuActionOptions | undefined,
  ancestors: Set<MenuId>,
): IAction | undefined {
  if (isMenuItem(item)) {
    const alt = item.alt
      ? new MenuItemAction(
        item.alt,
        undefined,
        options,
        contextKeyService,
        commandService,
      )
      : undefined;
    return new MenuItemAction(
      item.command,
      alt,
      options,
      contextKeyService,
      commandService,
    );
  }

  const groups = resolveMenu(
    item.submenu,
    commandService,
    contextKeyService,
    options,
    ancestors,
  );
  const actions = Separator.join(
    ...groups.map(([, groupActions]) => [...groupActions]),
  );
  return actions.length > 0
    ? new SubmenuItemAction(item, actions)
    : undefined;
}

function compareMenuItems(
  first: MenuRegistryItem,
  second: MenuRegistryItem,
): number {
  const groupComparison = compareGroups(first.group, second.group);
  if (groupComparison !== 0) return groupComparison;

  const orderComparison = (first.order ?? 0) - (second.order ?? 0);
  if (orderComparison !== 0) return orderComparison;

  return itemTitle(first).localeCompare(itemTitle(second));
}

function compareGroups(
  first: string | undefined,
  second: string | undefined,
): number {
  if (first === second) return 0;
  if (!first) return 1;
  if (!second) return -1;
  if (first === "navigation") return -1;
  if (second === "navigation") return 1;
  return first.localeCompare(second);
}

function itemTitle(item: MenuRegistryItem): string {
  if (!isMenuItem(item)) return item.title;
  return typeof item.command.title === "string"
    ? item.command.title
    : item.command.title.original;
}
