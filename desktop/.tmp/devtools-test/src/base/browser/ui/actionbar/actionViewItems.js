import { Separator, } from "../../../common/actions.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { Button } from "../button/button.js";
/**
 * Browser representation of one action inside an ActionBar.
 *
 * Implementations render into the container owned by their host and own the
 * resources they create for that representation.
 */
export class ActionViewItem extends DisposableOwner {
    action;
    constructor(action) {
        super();
        this.action = action;
    }
    focus() { }
}
/** Default button representation for a runnable action. */
export class ButtonActionViewItem extends ActionViewItem {
    #button;
    constructor(action) {
        super(action);
    }
    render(container) {
        if (this.#button) {
            throw new Error(`Action view item is already rendered: ${this.action.id}`);
        }
        this.#button = this.own(new Button({
            label: this.action.label,
            ownerDocument: container.ownerDocument,
            icon: this.action.icon,
            title: this.action.tooltip,
            enabled: this.action.enabled,
            checked: this.action.checked,
            onClick: () => this.runAction(),
        }));
        container.append(this.#button.element);
    }
    focus() {
        this.button.element.focus();
    }
    get button() {
        if (!this.#button) {
            throw new Error(`Action view item is not rendered: ${this.action.id}`);
        }
        return this.#button;
    }
    runAction() {
        return this.action.run();
    }
}
/** Non-interactive visual representation of a separator action. */
export class SeparatorActionViewItem extends ActionViewItem {
    #rendered = false;
    constructor(action) {
        super(action);
    }
    render(container) {
        if (this.#rendered) {
            throw new Error(`Action view item is already rendered: ${this.action.id}`);
        }
        this.#rendered = true;
        container.classList.add("zeta-action-view-item-separator");
        container.setAttribute("role", "separator");
    }
}
/** Creates the base representation used when a platform has no override. */
export function createActionViewItem(action) {
    return action instanceof Separator
        ? new SeparatorActionViewItem(action)
        : new ButtonActionViewItem(action);
}
