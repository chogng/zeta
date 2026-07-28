import { MenuWorkbenchToolBar, } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId, } from "../../../../platform/actions/common/actions.js";
import { WorkbenchPart } from "../../part.js";
import { BrowserMenubarControl, } from "./menubarControl.js";
/** The host-neutral workbench title area and its actions. */
export class BrowserTitlebarPart extends WorkbenchPart {
    #label;
    #menubar;
    #leftActions;
    #actions;
    get minimumHeight() { return 35; }
    get maximumHeight() { return 35; }
    constructor(options, menubar) {
        super("titlebar", options.ownerDocument);
        this.#label = options.ownerDocument.createElement("h1");
        this.#label.className = "zeta-titlebar-label";
        this.#label.textContent = options.title;
        this.#menubar = this.own(menubar);
        this.#leftActions = this.own(new MenuWorkbenchToolBar(options.menuService, options.contextMenuService, MenuId.TitleBarLeft, options.ownerDocument));
        this.#actions = this.own(new MenuWorkbenchToolBar(options.menuService, options.contextMenuService, MenuId.TitleBar, options.ownerDocument));
        const leftActionsElement = options.ownerDocument.createElement("div");
        leftActionsElement.className = "zeta-titlebar-left-actions";
        leftActionsElement.append(this.#leftActions.element);
        this.titleElement.append(leftActionsElement);
        if (this.#menubar.element) {
            this.titleElement.append(this.#menubar.element);
        }
        this.titleElement.append(this.#label);
        const actionsElement = options.ownerDocument.createElement("div");
        actionsElement.className = "zeta-titlebar-actions";
        actionsElement.append(this.#actions.element);
        this.contentElement.append(actionsElement);
    }
    setTitle(title) { this.#label.textContent = title; }
}
/** Creates the titlebar used by a regular web workbench. */
export const createBrowserTitlebarPart = (options) => new BrowserTitlebarPart(options, new BrowserMenubarControl(options.menuService, options.contextMenuService, options.ownerDocument));
