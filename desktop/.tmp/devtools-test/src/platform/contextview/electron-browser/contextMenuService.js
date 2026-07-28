import { isNode } from "../../../base/browser/dom.js";
import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner, } from "../../../base/common/lifecycle.js";
import { Separator, SubmenuAction, } from "../../../base/common/actions.js";
import { resolveContextMenuActions, } from "../browser/contextMenu.js";
/** macOS implementation backed by Electron's native Menu. */
export class NativeContextMenuService extends DisposableOwner {
    #onDidShowContextMenu = this.own(new Emitter());
    #onDidHideContextMenu = this.own(new Emitter());
    #api;
    #menuService;
    #keybindingService;
    #open = false;
    onDidShowContextMenu = this.#onDidShowContextMenu.event;
    onDidHideContextMenu = this.#onDidHideContextMenu.event;
    constructor(api, menuService, keybindingService) {
        super();
        this.#api = api;
        this.#menuService = menuService;
        this.#keybindingService = keybindingService;
        this.defer(() => this.hideContextMenu());
    }
    showContextMenu(options) {
        if (this.#open) {
            options.onHide?.(true);
            return;
        }
        const actions = resolveContextMenuActions(options, this.#menuService);
        const serialized = serializeActions(actions, this.#keybindingService);
        if (serialized.items.length === 0) {
            options.onHide?.(true);
            return;
        }
        const point = anchorPoint(options.anchor);
        const request = {
            items: serialized.items,
            x: point.x,
            y: point.y,
        };
        this.#open = true;
        this.#onDidShowContextMenu.fire();
        void this.#popup(request, serialized.actions, options.onHide);
    }
    hideContextMenu() {
        if (!this.#open)
            return;
        void this.#api.close().catch((error) => {
            console.error("Failed to close native context menu", error);
        });
    }
    async #popup(request, actions, onHide) {
        let selected;
        try {
            const result = await this.#api.popup(request);
            selected = result.selectedId
                ? actions.get(result.selectedId)
                : undefined;
        }
        catch (error) {
            console.error("Failed to show native context menu", error);
        }
        finally {
            this.#open = false;
            onHide?.(!selected);
            this.#onDidHideContextMenu.fire();
        }
        if (selected)
            runAction(selected);
    }
}
function serializeActions(actions, keybindingService) {
    const actionMap = new Map();
    let nextId = 1;
    const serialize = (source) => {
        const items = [];
        for (const action of source) {
            if (action instanceof Separator) {
                items.push({ type: "separator" });
                continue;
            }
            if (action instanceof SubmenuAction) {
                const children = serialize(action.actions);
                if (children.length > 0) {
                    items.push({
                        type: "submenu",
                        label: action.label,
                        enabled: action.enabled,
                        items: children,
                    });
                }
                continue;
            }
            const id = `action-${nextId++}`;
            actionMap.set(id, action);
            const accelerator = toElectronAccelerator(keybindingService.lookupKeybinding(action.id));
            items.push({
                type: "action",
                id,
                label: action.label,
                enabled: action.enabled,
                ...(accelerator ? { accelerator } : {}),
                ...(action.checked === undefined
                    ? {}
                    : { checked: action.checked }),
            });
        }
        return trimSerializedSeparators(items);
    };
    return {
        items: serialize(actions),
        actions: actionMap,
    };
}
function toElectronAccelerator(keybinding) {
    if (!keybinding || keybinding.chords.length !== 1)
        return undefined;
    const chord = keybinding.chords[0];
    const key = electronKey(chord);
    if (!key)
        return undefined;
    const parts = [];
    if (chord.metaKey)
        parts.push("Command");
    if (chord.ctrlKey)
        parts.push("Control");
    if (chord.altKey)
        parts.push("Alt");
    if (chord.shiftKey)
        parts.push("Shift");
    parts.push(key);
    return parts.join("+");
}
function electronKey(chord) {
    const key = chord.label ?? chord.key;
    if (/^[a-z0-9]$/i.test(key))
        return key.toUpperCase();
    if (/^Key[A-Z]$/.test(key))
        return key.slice(3);
    if (/^Digit[0-9]$/.test(key))
        return key.slice(5);
    if (/^F(?:[1-9]|1[0-9]|2[0-4])$/i.test(key)) {
        return key.toUpperCase();
    }
    const knownKeys = {
        " ": "Space",
        arrowdown: "Down",
        arrowleft: "Left",
        arrowright: "Right",
        arrowup: "Up",
        backspace: "Backspace",
        delete: "Delete",
        end: "End",
        enter: "Enter",
        escape: "Escape",
        home: "Home",
        pagedown: "PageDown",
        pageup: "PageUp",
        space: "Space",
        tab: "Tab",
    };
    return knownKeys[key.toLocaleLowerCase("en-US")];
}
function trimSerializedSeparators(items) {
    const result = [];
    for (const item of items) {
        if (item.type === "separator" &&
            (result.length === 0 || result[result.length - 1]?.type === "separator")) {
            continue;
        }
        result.push(item);
    }
    if (result[result.length - 1]?.type === "separator")
        result.pop();
    return result;
}
function anchorPoint(anchor) {
    if (!isNode(anchor)) {
        return {
            x: normalizeCoordinate(anchor.x),
            y: normalizeCoordinate(anchor.y),
        };
    }
    const bounds = anchor.getBoundingClientRect();
    return {
        x: normalizeCoordinate(bounds.left),
        y: normalizeCoordinate(bounds.bottom),
    };
}
function normalizeCoordinate(value) {
    return Math.max(-1_000_000, Math.min(1_000_000, Math.round(value)));
}
function runAction(action) {
    try {
        Promise.resolve(action.run()).catch((error) => {
            console.error(`Context menu action failed: ${action.id}`, error);
        });
    }
    catch (error) {
        console.error(`Context menu action failed: ${action.id}`, error);
    }
}
