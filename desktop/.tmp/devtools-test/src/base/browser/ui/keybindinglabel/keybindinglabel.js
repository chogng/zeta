import { getKeybindingLabel, getKeybindingLabelParts, KeybindingLabelStyle, } from "../../../common/keybindingLabels.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";
/** Presents a resolved keybinding without owning matching or dispatch policy. */
export class KeybindingLabel extends DisposableOwner {
    element;
    #keybinding;
    constructor(options) {
        super();
        const ownerDocument = options.ownerDocument ?? document;
        this.#keybinding = options.keybinding;
        this.element = ownerDocument.createElement("span");
        this.defer(() => this.element.remove());
        this.element.className = "zeta-keybinding-label";
        this.#render();
    }
    set keybinding(keybinding) {
        this.#keybinding = keybinding;
        this.#render();
    }
    get keybinding() {
        return this.#keybinding;
    }
    #render() {
        const ownerDocument = this.element.ownerDocument;
        const parts = getKeybindingLabelParts(this.#keybinding);
        this.element.replaceChildren(...parts.map((part) => {
            const token = ownerDocument.createElement("kbd");
            token.textContent = part.label;
            setAriaAttribute(token, "label", part.ariaLabel);
            return token;
        }));
        setAriaAttribute(this.element, "label", getKeybindingLabel(this.#keybinding, KeybindingLabelStyle.Aria));
    }
}
