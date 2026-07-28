import { SubmenuAction, } from "../../../base/common/actions.js";
import { Emitter } from "../../../base/common/event.js";
import { DisposableStore, toDisposable, } from "../../../base/common/lifecycle.js";
import { commandActionLabel, isCommandActionToggleInfo, } from "../../action/common/action.js";
import { CommandsRegistry, } from "../../commands/common/commands.js";
import { ContextKeyExpr, } from "../../contextkey/common/contextkey.js";
import { KeybindingsRegistry, } from "../../keybinding/common/keybindingsRegistry.js";
export function isMenuItem(item) {
    return "command" in item;
}
/** Identifies an action contribution location, regardless of its final UI. */
export class MenuId {
    static #instances = new Map();
    static CommandPalette = new MenuId("CommandPalette");
    static TitleBar = new MenuId("TitleBar");
    static TitleBarLeft = new MenuId("TitleBarLeft");
    static MenubarMainMenu = new MenuId("MenubarMainMenu");
    static MenubarFileMenu = new MenuId("MenubarFileMenu");
    static MenubarEditMenu = new MenuId("MenubarEditMenu");
    static MenubarSelectionMenu = new MenuId("MenubarSelectionMenu");
    static MenubarViewMenu = new MenuId("MenubarViewMenu");
    static MenubarGoMenu = new MenuId("MenubarGoMenu");
    static MenubarRunMenu = new MenuId("MenubarRunMenu");
    static MenubarTerminalMenu = new MenuId("MenubarTerminalMenu");
    static MenubarHelpMenu = new MenuId("MenubarHelpMenu");
    static for(identifier) {
        return this.#instances.get(identifier) ?? new MenuId(identifier);
    }
    id;
    constructor(identifier) {
        if (MenuId.#instances.has(identifier)) {
            throw new TypeError(`MenuId '${identifier}' already exists; use MenuId.for()`);
        }
        this.id = identifier;
        MenuId.#instances.set(identifier, this);
    }
}
/** Realm-wide registry of static and dynamic action placements. */
export class MenuRegistry {
    #items = new Map();
    #onDidChangeMenu = new Emitter();
    onDidChangeMenu = this.#onDidChangeMenu.event;
    appendMenuItem(id, item) {
        let items = this.#items.get(id);
        if (!items) {
            items = [];
            this.#items.set(id, items);
        }
        items.push(item);
        this.#onDidChangeMenu.fire({ menuId: id });
        return toDisposable(() => {
            const current = this.#items.get(id);
            if (!current)
                return;
            const index = current.indexOf(item);
            if (index < 0)
                return;
            current.splice(index, 1);
            if (current.length === 0)
                this.#items.delete(id);
            this.#onDidChangeMenu.fire({ menuId: id });
        });
    }
    getMenuItems(id) {
        return [...(this.#items.get(id) ?? [])];
    }
}
/** Realm-wide menu contributions populated by static contribution modules. */
export const MenusRegistry = new MenuRegistry();
/** A command contribution resolved into a runnable UI action. */
export class MenuItemAction {
    item;
    alt;
    id;
    label;
    tooltip;
    icon;
    enabled;
    checked;
    #options;
    #commandService;
    constructor(item, alt, options, contextKeyService, commandService) {
        this.item = item;
        this.alt = alt;
        this.#options = options;
        this.#commandService = commandService;
        this.id = item.id;
        this.label = options?.renderShortTitle && item.shortTitle
            ? commandActionLabel(item.shortTitle)
            : commandActionLabel(item.title);
        this.tooltip = item.tooltip
            ? commandActionLabel(item.tooltip)
            : this.label;
        this.icon = item.icon;
        this.enabled = contextKeyService.contextMatchesRules(item.precondition);
        if (item.toggled) {
            const toggled = isCommandActionToggleInfo(item.toggled)
                ? item.toggled
                : { condition: item.toggled };
            this.checked = contextKeyService.contextMatchesRules(toggled.condition);
            if (this.checked) {
                if (toggled.title)
                    this.label = commandActionLabel(toggled.title);
                if (toggled.tooltip) {
                    this.tooltip = commandActionLabel(toggled.tooltip);
                }
                if (toggled.icon)
                    this.icon = toggled.icon;
            }
        }
    }
    run(...args) {
        const commandArgs = [];
        if (this.#options?.args) {
            commandArgs.push(...this.#options.args);
        }
        else if (this.#options && "arg" in this.#options) {
            commandArgs.push(this.#options.arg);
        }
        if (this.#options?.shouldForwardArgs)
            commandArgs.push(...args);
        return this.#commandService.executeCommand(this.id, ...commandArgs);
    }
}
/** A submenu contribution resolved into a nested runtime action. */
export class SubmenuItemAction extends SubmenuAction {
    item;
    constructor(item, actions) {
        super(`submenu.${item.submenu.id}`, item.title, actions);
        this.item = item;
    }
}
/** Base class for a statically declared command and its UI contributions. */
export class Action2 {
    desc;
    constructor(desc) {
        this.desc = desc;
    }
}
/**
 * Registers a built-in action for the current JavaScript realm.
 *
 * Static contribution modules intentionally keep registrations for the realm
 * lifetime. Dynamic callers must retain and dispose the returned registration.
 */
export function registerAction2(ctor) {
    const action = new ctor();
    const registrations = new DisposableStore();
    try {
        registrations.add(CommandsRegistry.register(action.desc.id, (accessor, ...args) => action.run(accessor, ...args)));
        if (action.desc.keybinding) {
            const contribution = action.desc.keybinding;
            const keybindings = [
                contribution.primary,
                ...(contribution.secondary ?? []),
            ];
            const when = ContextKeyExpr.and(action.desc.precondition, contribution.when);
            for (const keybinding of keybindings) {
                registrations.add(KeybindingsRegistry.registerKeybindingRule({
                    command: action.desc.id,
                    keybinding,
                    when,
                    args: contribution.args,
                    weight: contribution.weight,
                }));
            }
        }
        const placements = action.desc.menu
            ? Array.isArray(action.desc.menu)
                ? action.desc.menu
                : [action.desc.menu]
            : [];
        for (const placement of placements) {
            registrations.add(MenusRegistry.appendMenuItem(placement.id, {
                command: action.desc,
                when: placement.when,
                group: placement.group,
                order: placement.order,
            }));
        }
        if (action.desc.f1) {
            registrations.add(MenusRegistry.appendMenuItem(MenuId.CommandPalette, {
                command: action.desc,
                when: action.desc.precondition,
            }));
        }
    }
    catch (error) {
        registrations.dispose();
        throw error;
    }
    return registrations;
}
