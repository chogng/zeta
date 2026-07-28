import { Separator, } from "../../../base/common/actions.js";
import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
import { isMenuItem, MenuItemAction, MenusRegistry, SubmenuItemAction, } from "./actions.js";
export const IMenuService = createServiceIdentifier("menuService");
/** Resolves registered menu contributions for one workbench context. */
export class MenuService {
    #commandService;
    #contextKeyService;
    constructor(commandService, contextKeyService) {
        this.#commandService = commandService;
        this.#contextKeyService = contextKeyService;
    }
    createMenu(id) {
        return new Menu(id, this.#commandService, this.#contextKeyService);
    }
    getMenuActions(id, options) {
        return resolveMenu(id, this.#commandService, this.#contextKeyService, options, new Set());
    }
}
class Menu extends DisposableOwner {
    #onDidChange = this.own(new Emitter());
    onDidChange = this.#onDidChange.event;
    #id;
    #commandService;
    #contextKeyService;
    constructor(id, commandService, contextKeyService) {
        super();
        this.#id = id;
        this.#commandService = commandService;
        this.#contextKeyService = contextKeyService;
        this.own(MenusRegistry.onDidChangeMenu((_event) => {
            this.#onDidChange.fire();
        }));
        this.own(this.#contextKeyService.onDidChangeContext(() => {
            this.#onDidChange.fire();
        }));
    }
    getActions(options) {
        return resolveMenu(this.#id, this.#commandService, this.#contextKeyService, options, new Set());
    }
}
function resolveMenu(id, commandService, contextKeyService, options, ancestors) {
    if (ancestors.has(id)) {
        throw new Error(`Menu contribution cycle detected at '${id.id}'`);
    }
    const nextAncestors = new Set(ancestors);
    nextAncestors.add(id);
    const sorted = [...MenusRegistry.getMenuItems(id)].sort(compareMenuItems);
    const groups = new Map();
    for (const item of sorted) {
        if (!contextKeyService.contextMatchesRules(item.when))
            continue;
        const action = resolveItem(item, commandService, contextKeyService, options, nextAncestors);
        if (!action)
            continue;
        const group = item.group ?? "";
        const actions = groups.get(group);
        if (actions)
            actions.push(action);
        else
            groups.set(group, [action]);
    }
    return [...groups].map(([group, actions]) => [group, actions]);
}
function resolveItem(item, commandService, contextKeyService, options, ancestors) {
    if (isMenuItem(item)) {
        const alt = item.alt
            ? new MenuItemAction(item.alt, undefined, options, contextKeyService, commandService)
            : undefined;
        return new MenuItemAction(item.command, alt, options, contextKeyService, commandService);
    }
    const groups = resolveMenu(item.submenu, commandService, contextKeyService, options, ancestors);
    const actions = Separator.join(...groups.map(([, groupActions]) => [...groupActions]));
    return actions.length > 0 || options?.preserveEmptySubmenus
        ? new SubmenuItemAction(item, actions)
        : undefined;
}
function compareMenuItems(first, second) {
    const groupComparison = compareGroups(first.group, second.group);
    if (groupComparison !== 0)
        return groupComparison;
    const orderComparison = (first.order ?? 0) - (second.order ?? 0);
    if (orderComparison !== 0)
        return orderComparison;
    return itemTitle(first).localeCompare(itemTitle(second));
}
function compareGroups(first, second) {
    if (first === second)
        return 0;
    if (!first)
        return 1;
    if (!second)
        return -1;
    if (first === "navigation")
        return -1;
    if (second === "navigation")
        return 1;
    return first.localeCompare(second);
}
function itemTitle(item) {
    if (!isMenuItem(item))
        return item.title;
    return typeof item.command.title === "string"
        ? item.command.title
        : item.command.title.original;
}
