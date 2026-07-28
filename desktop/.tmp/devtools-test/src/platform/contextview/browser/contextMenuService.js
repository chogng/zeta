import { Emitter } from "../../../base/common/event.js";
import { isNode } from "../../../base/browser/dom.js";
import { DisposableOwner, DisposableSlot, } from "../../../base/common/lifecycle.js";
import { AnchorPosition, ContextView, ContextViewFocusRestore, } from "../../../base/browser/ui/contextview/contextview.js";
import { Menu } from "../../../base/browser/ui/menu/menu.js";
import { resolveContextMenuActions, } from "./contextMenu.js";
/** HTML implementation used by web, Windows, and Linux workbenches. */
export class BrowserContextMenuService extends DisposableOwner {
    #onDidShowContextMenu = this.own(new Emitter());
    #onDidHideContextMenu = this.own(new Emitter());
    #activeMenu = this.own(new DisposableSlot());
    #contextView;
    #menuService;
    #keybindingService;
    #onHide;
    #didSelect = false;
    #active = false;
    #shown = false;
    onDidShowContextMenu = this.#onDidShowContextMenu.event;
    onDidHideContextMenu = this.#onDidHideContextMenu.event;
    constructor(menuService, keybindingService, ownerDocument) {
        super();
        this.#menuService = menuService;
        this.#keybindingService = keybindingService;
        this.#contextView = this.own(new ContextView(ownerDocument));
        this.defer(() => this.hideContextMenu());
    }
    showContextMenu(options) {
        this.hideContextMenu();
        const actions = resolveContextMenuActions(options, this.#menuService);
        if (actions.length === 0) {
            options.onHide?.(true);
            return;
        }
        this.#didSelect = false;
        this.#active = true;
        this.#shown = false;
        this.#onHide = options.onHide;
        const menu = new Menu({
            actions,
            ownerDocument: this.#contextView.element.ownerDocument,
            layer: 10,
            getKeybinding: (action) => this.#keybindingService.lookupKeybinding(action.id),
            onDidSelect: () => {
                this.#didSelect = true;
                this.#contextView.hide();
            },
        });
        this.#activeMenu.replace(menu);
        const shown = this.#contextView.show({
            anchor: toContextViewAnchor(options.anchor),
            content: menu.element,
            anchorPosition: AnchorPosition.Below,
            focusRestore: ContextViewFocusRestore.Previous,
            layer: 10,
            isTargetWithin: (target) => menu.contains(target),
            onHide: () => this.#didHide(),
        });
        if (!shown) {
            this.#didHide();
            return;
        }
        this.#shown = true;
        menu.focusFirst();
        this.#onDidShowContextMenu.fire();
    }
    hideContextMenu() {
        if (this.#contextView.visible) {
            this.#contextView.hide();
        }
        else if (this.#activeMenu.value) {
            this.#didHide();
        }
    }
    #didHide() {
        if (!this.#active)
            return;
        this.#active = false;
        const onHide = this.#onHide;
        const didCancel = !this.#didSelect;
        const shown = this.#shown;
        this.#onHide = undefined;
        this.#didSelect = false;
        this.#shown = false;
        this.#activeMenu.clear();
        onHide?.(didCancel);
        if (shown)
            this.#onDidHideContextMenu.fire();
    }
}
function toContextViewAnchor(anchor) {
    if (!isNode(anchor)) {
        return {
            left: anchor.x,
            top: anchor.y,
            width: 0,
            height: 0,
        };
    }
    return anchor;
}
