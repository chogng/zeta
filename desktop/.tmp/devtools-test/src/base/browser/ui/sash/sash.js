import { addDisposableListener } from "../../dom.js";
import { getWindow } from "../../window.js";
import { DisposableOwner, ResettableDisposableGroup, toDisposable, } from "../../../common/lifecycle.js";
/** A draggable and keyboard-operable separator owned by a layout control. */
export class Sash extends DisposableOwner {
    orientation;
    element;
    #startListeners = new Set();
    #changeListeners = new Set();
    #endListeners = new Set();
    #dragListeners;
    constructor(orientation, ownerDocument = document) {
        super();
        this.orientation = orientation;
        const element = ownerDocument.createElement("div");
        this.element = element;
        this.defer(() => element.remove());
        element.className = `zeta-sash zeta-sash-${orientation}`;
        element.setAttribute("role", "separator");
        element.setAttribute("aria-orientation", orientation);
        element.tabIndex = 0;
        this.#dragListeners = this.own(new ResettableDisposableGroup());
        this.own(toDisposable(() => {
            this.#startListeners.clear();
            this.#changeListeners.clear();
            this.#endListeners.clear();
        }));
        this.own(addDisposableListener(element, "pointerdown", (event) => this.beginDrag(event)));
        this.own(addDisposableListener(element, "keydown", (event) => this.handleKeydown(event)));
    }
    onDidStart(listener) {
        this.#startListeners.add(listener);
        return toDisposable(() => this.#startListeners.delete(listener));
    }
    onDidChange(listener) {
        this.#changeListeners.add(listener);
        return toDisposable(() => this.#changeListeners.delete(listener));
    }
    onDidEnd(listener) {
        this.#endListeners.add(listener);
        return toDisposable(() => this.#endListeners.delete(listener));
    }
    beginDrag(event) {
        if (event.button !== 0)
            return;
        event.preventDefault();
        this.#dragListeners.clear();
        const start = this.coordinate(event);
        this.fire(this.#startListeners);
        if (typeof event.pointerId === "number" &&
            typeof this.element.setPointerCapture === "function") {
            this.element.setPointerCapture(event.pointerId);
        }
        const move = (next) => {
            const dragEvent = { delta: this.coordinate(next) - start };
            for (const listener of this.#changeListeners)
                listener(dragEvent);
        };
        const stop = () => {
            this.#dragListeners.clear();
            if (typeof event.pointerId === "number" &&
                typeof this.element.hasPointerCapture === "function" &&
                this.element.hasPointerCapture(event.pointerId)) {
                this.element.releasePointerCapture(event.pointerId);
            }
            this.fire(this.#endListeners);
        };
        const targetWindow = getWindow(this.element);
        this.#dragListeners.add(addDisposableListener(targetWindow, "pointermove", move));
        this.#dragListeners.add(addDisposableListener(targetWindow, "pointerup", stop, { once: true }));
        this.#dragListeners.add(addDisposableListener(targetWindow, "pointercancel", stop, { once: true }));
        this.#dragListeners.add(addDisposableListener(targetWindow, "blur", stop, { once: true }));
    }
    handleKeydown(event) {
        const delta = this.keyboardDelta(event);
        if (delta === undefined)
            return;
        event.preventDefault();
        this.fire(this.#startListeners);
        for (const listener of this.#changeListeners)
            listener({ delta });
        this.fire(this.#endListeners);
    }
    coordinate(event) {
        return this.orientation === "vertical" ? event.clientX : event.clientY;
    }
    keyboardDelta(event) {
        const step = event.altKey ? 1 : 10;
        if (this.orientation === "vertical") {
            if (event.key === "ArrowLeft")
                return -step;
            if (event.key === "ArrowRight")
                return step;
            return undefined;
        }
        if (event.key === "ArrowUp")
            return -step;
        if (event.key === "ArrowDown")
            return step;
        return undefined;
    }
    fire(listeners) {
        for (const listener of listeners)
            listener();
    }
}
