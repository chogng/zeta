import { DisposableOwner, ResettableDisposableGroup, } from "../../../common/lifecycle.js";
import { createActionViewItem, } from "./actionViewItems.js";
/** Owns and arranges action view items without interpreting action subtypes. */
export class ActionBar extends DisposableOwner {
    element;
    #items = this.own(new ResettableDisposableGroup());
    #actionViewItemProvider;
    constructor(options = {}) {
        super();
        const ownerDocument = options.ownerDocument ?? document;
        const element = ownerDocument.createElement("div");
        this.element = element;
        this.defer(() => element.remove());
        element.className = "zeta-action-bar";
        element.setAttribute("role", "toolbar");
        this.#actionViewItemProvider = options.actionViewItemProvider;
        this.setActions(options.actions ?? []);
    }
    add(action) {
        const container = this.element.ownerDocument.createElement("div");
        container.className = "zeta-action-view-item";
        container.dataset.actionId = action.id;
        container.setAttribute("role", "presentation");
        const item = this.#items.add(this.#actionViewItemProvider?.(action) ??
            createActionViewItem(action));
        this.element.append(container);
        item.render(container);
        return item;
    }
    setActions(actions) {
        this.#items.clear();
        this.element.replaceChildren();
        for (const action of actions)
            this.add(action);
    }
}
