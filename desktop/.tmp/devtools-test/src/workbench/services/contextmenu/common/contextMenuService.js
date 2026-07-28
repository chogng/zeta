import { DisposableOwner, } from "../../../../base/common/lifecycle.js";
/**
 * Workbench-owned context menu facade.
 *
 * Host entry points choose the rendering implementation while consumers use
 * one stable service contract. The facade owns that implementation for the
 * lifetime of the workbench window.
 */
export class WorkbenchContextMenuService extends DisposableOwner {
    #implementation;
    onDidShowContextMenu;
    onDidHideContextMenu;
    constructor(implementation) {
        super();
        this.#implementation = this.own(implementation);
        this.onDidShowContextMenu = implementation.onDidShowContextMenu;
        this.onDidHideContextMenu = implementation.onDidHideContextMenu;
    }
    showContextMenu(options) {
        this.#implementation.showContextMenu(options);
    }
    hideContextMenu() {
        this.#implementation.hideContextMenu();
    }
}
