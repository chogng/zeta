import { addDisposableListener } from "../../dom.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";
import { appendIcon } from "../icon/icon.js";
/** A semantic button with an explicit enabled state. */
export class Button extends DisposableOwner {
    element;
    constructor(options) {
        super();
        const ownerDocument = options.ownerDocument ?? document;
        const element = ownerDocument.createElement("button");
        this.element = element;
        this.defer(() => element.remove());
        element.className = "zeta-button";
        element.type = "button";
        if (options.icon)
            appendIcon(options.icon, element);
        const label = ownerDocument.createElement("span");
        label.textContent = options.label;
        element.append(label);
        element.title = options.title ?? options.label;
        element.disabled = options.enabled === false;
        if (options.checked !== undefined) {
            setAriaAttribute(element, "pressed", options.checked);
        }
        if (options.onClick) {
            this.own(addDisposableListener(element, "click", options.onClick));
        }
    }
    set enabled(value) { this.element.disabled = !value; }
    get enabled() { return !this.element.disabled; }
}
