import { DisposableOwner, DisposableSlot, } from "../../../common/lifecycle.js";
import { scheduleAtNextAnimationFrame } from "../../scheduler.js";
/** Sets or removes one ARIA attribute while preserving boolean false. */
export function setAriaAttribute(element, attribute, value) {
    const name = `aria-${attribute}`;
    if (value === undefined || value === null) {
        element.removeAttribute(name);
    }
    else {
        element.setAttribute(name, String(value));
    }
}
/** Reads one ARIA attribute without exposing DOM null semantics. */
export function getAriaAttribute(element, attribute) {
    return element.getAttribute(`aria-${attribute}`) ?? undefined;
}
/** Sets or removes an element's semantic role. */
export function setRole(element, role) {
    if (role === undefined) {
        element.removeAttribute("role");
    }
    else {
        element.setAttribute("role", role);
    }
}
/**
 * Owns screen-reader status and alert regions for one document.
 *
 * Callers should create one region per UI root and dispose it with that root.
 */
export class AriaLiveRegion extends DisposableOwner {
    #root;
    #polite;
    #assertive;
    #pending = this.own(new DisposableSlot());
    #politeIndex = 0;
    #assertiveIndex = 0;
    constructor(ownerDocument) {
        super();
        this.#root = ownerDocument.createElement("div");
        this.#root.className = "zeta-aria-live";
        this.#polite = [
            this.#createRegion(ownerDocument, "polite"),
            this.#createRegion(ownerDocument, "polite"),
        ];
        this.#assertive = [
            this.#createRegion(ownerDocument, "assertive"),
            this.#createRegion(ownerDocument, "assertive"),
        ];
        this.#root.append(...this.#polite, ...this.#assertive);
        ownerDocument.body.append(this.#root);
        this.defer(() => this.#root.remove());
    }
    status(message) {
        this.announce(message, "polite");
    }
    alert(message) {
        this.announce(message, "assertive");
    }
    announce(message, priority = "polite") {
        const regions = priority === "assertive"
            ? this.#assertive
            : this.#polite;
        const index = priority === "assertive"
            ? this.#assertiveIndex
            : this.#politeIndex;
        const target = regions[index];
        const alternate = regions[index === 0 ? 1 : 0];
        if (priority === "assertive") {
            this.#assertiveIndex = index === 0 ? 1 : 0;
        }
        else {
            this.#politeIndex = index === 0 ? 1 : 0;
        }
        this.#pending.clear();
        target.textContent = "";
        alternate.textContent = "";
        const targetWindow = target.ownerDocument.defaultView;
        if (!targetWindow)
            return;
        this.#pending.replace(scheduleAtNextAnimationFrame(targetWindow, () => {
            this.#pending.clear();
            target.textContent = message.slice(0, maximumMessageLength);
        }));
    }
    clear() {
        this.#pending.clear();
        for (const region of [...this.#polite, ...this.#assertive]) {
            region.textContent = "";
        }
    }
    #createRegion(ownerDocument, priority) {
        const region = ownerDocument.createElement("div");
        region.className = priority === "assertive"
            ? "zeta-aria-alert"
            : "zeta-aria-status";
        if (priority === "assertive") {
            setRole(region, "alert");
        }
        else {
            setAriaAttribute(region, "live", "polite");
        }
        setAriaAttribute(region, "atomic", true);
        return region;
    }
}
const maximumMessageLength = 20_000;
