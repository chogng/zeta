import { addDisposableListener, stopEvent } from "../../dom.js";
import { Emitter } from "../../../common/event.js";
import { LxIcon } from "../../../common/lxicons.js";
import { ActionViewItem } from "../actionbar/actionViewItems.js";
import { Button } from "../button/button.js";
import { appendIcon } from "../icon/icon.js";
/**
 * ActionBar item that delegates a dropdown action menu to the host provider.
 *
 * The provider owns menu rendering, positioning, native/browser selection, and
 * dismissal. This item owns only its trigger and expanded accessibility state.
 */
export class DropdownMenuActionViewItem extends ActionViewItem {
    #actions;
    #contextMenuProvider;
    #onDidChangeVisibility = this.own(new Emitter());
    onDidChangeVisibility = this.#onDidChangeVisibility.event;
    #button;
    #visible = false;
    constructor(action, actions, contextMenuProvider) {
        super(action);
        this.#actions = actions;
        this.#contextMenuProvider = contextMenuProvider;
    }
    render(container) {
        if (this.#button) {
            throw new Error(`Action view item is already rendered: ${this.action.id}`);
        }
        const button = this.own(new Button({
            label: this.action.label,
            ownerDocument: container.ownerDocument,
            icon: this.action.icon,
            title: this.action.tooltip,
            enabled: this.action.enabled,
            onClick: () => this.show(),
        }));
        this.#button = button;
        container.classList.add("zeta-dropdown-menu-action-view-item");
        button.element.setAttribute("aria-haspopup", "menu");
        button.element.setAttribute("aria-expanded", "false");
        const indicator = container.ownerDocument.createElement("span");
        indicator.className = "zeta-dropdown-menu-indicator";
        appendIcon(LxIcon.dropdownIndicator, indicator);
        button.element.append(indicator);
        container.append(button.element);
        this.own(addDisposableListener(button.element, "keydown", (event) => {
            if (event.key !== "ArrowDown" && event.key !== "ArrowUp")
                return;
            stopEvent(event);
            this.show();
        }));
    }
    focus() {
        this.#button?.element.focus();
    }
    show() {
        const button = this.#button;
        if (!button?.enabled || this.#visible)
            return;
        const actions = typeof this.#actions === "function"
            ? this.#actions()
            : this.#actions;
        if (actions.length === 0)
            return;
        this.#setVisible(true);
        const options = {
            anchor: button.element,
            actions,
            onHide: () => this.#setVisible(false),
        };
        try {
            this.#contextMenuProvider.showContextMenu(options);
        }
        catch (error) {
            this.#setVisible(false);
            throw error;
        }
    }
    #setVisible(visible) {
        if (this.#visible === visible)
            return;
        this.#visible = visible;
        this.#button?.element.setAttribute("aria-expanded", String(visible));
        this.#onDidChangeVisibility.fire(visible);
    }
}
