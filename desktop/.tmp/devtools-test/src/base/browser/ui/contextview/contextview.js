import { Emitter } from "../../../common/event.js";
import { DisposableOwner, ResettableDisposableGroup, } from "../../../common/lifecycle.js";
import { AnchorAlignment, AnchorPosition, layout2d, } from "../../../common/layout.js";
import { addDisposableListener, isHTMLElement, isNode, } from "../../dom.js";
import { getActiveElement, restoreFocus, } from "../../focus.js";
import { getViewport } from "../../geometry.js";
import { getWindow, mainWindow } from "../../window.js";
export { AnchorAlignment, AnchorAxisAlignment, AnchorPosition, } from "../../../common/layout.js";
export var ContextViewFocusRestore;
(function (ContextViewFocusRestore) {
    ContextViewFocusRestore[ContextViewFocusRestore["None"] = 0] = "None";
    ContextViewFocusRestore[ContextViewFocusRestore["Previous"] = 1] = "Previous";
})(ContextViewFocusRestore || (ContextViewFocusRestore = {}));
export var ContextViewHideReason;
(function (ContextViewHideReason) {
    ContextViewHideReason[ContextViewHideReason["Programmatic"] = 0] = "Programmatic";
    ContextViewHideReason[ContextViewHideReason["Replaced"] = 1] = "Replaced";
    ContextViewHideReason[ContextViewHideReason["OutsidePointer"] = 2] = "OutsidePointer";
    ContextViewHideReason[ContextViewHideReason["Escape"] = 3] = "Escape";
    ContextViewHideReason[ContextViewHideReason["WindowBlur"] = 4] = "WindowBlur";
    ContextViewHideReason[ContextViewHideReason["AnchorRemoved"] = 5] = "AnchorRemoved";
})(ContextViewHideReason || (ContextViewHideReason = {}));
const visibleContextViews = new WeakMap();
/** An anchored, transient host for menus, hovers, and other overlays. */
export class ContextView extends DisposableOwner {
    element;
    #onDidHide = this.own(new Emitter());
    onDidHide = this.#onDidHide.event;
    #visibleListeners = this.own(new ResettableDisposableGroup());
    #restoreFocusTo;
    #options;
    constructor(ownerDocument = mainWindow.document) {
        super();
        const element = ownerDocument.createElement("div");
        this.element = element;
        this.defer(() => element.remove());
        element.className = "zeta-context-view";
        element.hidden = true;
        ownerDocument.body.append(element);
        this.defer(() => this.hide());
    }
    get visible() {
        return this.#options !== undefined;
    }
    show(options) {
        if (this.visible) {
            this.hide(ContextViewHideReason.Replaced);
        }
        this.#visibleListeners.clear();
        const ownerDocument = getAnchorDocument(options.anchor, this.element.ownerDocument);
        const targetWindow = getWindow(ownerDocument);
        if (this.element.ownerDocument !== ownerDocument) {
            ownerDocument.adoptNode(this.element);
            ownerDocument.body.append(this.element);
        }
        const activeElement = getActiveElement(ownerDocument);
        this.#restoreFocusTo = isHTMLElement(activeElement)
            ? activeElement
            : undefined;
        this.#options = options;
        this.element.replaceChildren(options.content);
        this.element.className = "zeta-context-view";
        this.element.style.zIndex = String(1000 + (options.layer ?? 0));
        this.element.style.visibility = "hidden";
        this.element.hidden = false;
        this.layout();
        if (!this.visible)
            return false;
        this.element.style.visibility = "";
        registerVisibleContextView(this);
        this.#visibleListeners.add(addDisposableListener(ownerDocument, "pointerdown", (event) => {
            const target = event.target;
            if (isNode(target) &&
                !this.element.contains(target) &&
                !anchorContains(options.anchor, target) &&
                !options.isTargetWithin?.(target)) {
                this.hide(ContextViewHideReason.OutsidePointer);
            }
        }, true));
        this.#visibleListeners.add(addDisposableListener(ownerDocument, "keydown", (event) => {
            if (event.isComposing ||
                event.key !== "Escape" ||
                !isTopmostContextView(this)) {
                return;
            }
            event.preventDefault();
            event.stopPropagation();
            this.hide(ContextViewHideReason.Escape);
        }, true));
        this.#visibleListeners.add(addDisposableListener(targetWindow, "blur", () => this.hide(ContextViewHideReason.WindowBlur)));
        this.#visibleListeners.add(addDisposableListener(targetWindow, "resize", () => this.layout()));
        this.#visibleListeners.add(addDisposableListener(ownerDocument, "scroll", () => this.layout(), true));
        return true;
    }
    layout() {
        const options = this.#options;
        if (!options)
            return;
        if (isElementAnchor(options.anchor) && !options.anchor.isConnected) {
            this.hide(ContextViewHideReason.AnchorRemoved);
            return;
        }
        const targetWindow = getWindow(this.element);
        const anchor = getAnchorRectangle(options.anchor);
        const bounds = this.element.getBoundingClientRect();
        const result = layout2d(getViewport(targetWindow), { width: bounds.width, height: bounds.height }, anchor, options);
        this.element.classList.toggle("zeta-context-view-above", result.anchorPosition === AnchorPosition.Above);
        this.element.classList.toggle("zeta-context-view-below", result.anchorPosition === AnchorPosition.Below);
        this.element.classList.toggle("zeta-context-view-align-right", result.anchorAlignment === AnchorAlignment.Right);
        this.element.classList.toggle("zeta-context-view-align-left", result.anchorAlignment === AnchorAlignment.Left);
        this.element.style.left = `${result.left}px`;
        this.element.style.top = `${result.top}px`;
    }
    hide(reason = ContextViewHideReason.Programmatic) {
        const options = this.#options;
        if (!options)
            return;
        this.#options = undefined;
        unregisterVisibleContextView(this);
        this.#visibleListeners.clear();
        this.element.hidden = true;
        this.element.replaceChildren();
        const restoreFocusTo = this.#restoreFocusTo;
        this.#restoreFocusTo = undefined;
        if (options.focusRestore === ContextViewFocusRestore.Previous &&
            restoreFocusTo) {
            restoreFocus(restoreFocusTo);
        }
        options.onHide?.(reason);
        this.#onDidHide.fire(reason);
    }
}
function registerVisibleContextView(contextView) {
    const ownerDocument = contextView.element.ownerDocument;
    const stack = visibleContextViews.get(ownerDocument);
    if (stack)
        stack.push(contextView);
    else
        visibleContextViews.set(ownerDocument, [contextView]);
}
function unregisterVisibleContextView(contextView) {
    const ownerDocument = contextView.element.ownerDocument;
    const stack = visibleContextViews.get(ownerDocument);
    if (!stack)
        return;
    const index = stack.lastIndexOf(contextView);
    if (index >= 0)
        stack.splice(index, 1);
    if (stack.length === 0)
        visibleContextViews.delete(ownerDocument);
}
function isTopmostContextView(contextView) {
    const stack = visibleContextViews.get(contextView.element.ownerDocument);
    return stack?.[stack.length - 1] === contextView;
}
function getAnchorDocument(anchor, fallback) {
    return isElementAnchor(anchor) ? anchor.ownerDocument : fallback;
}
function getAnchorRectangle(anchor) {
    if (!isElementAnchor(anchor))
        return anchor;
    const bounds = anchor.getBoundingClientRect();
    return {
        left: bounds.left,
        top: bounds.top,
        width: bounds.width,
        height: bounds.height,
    };
}
function anchorContains(anchor, target) {
    return isElementAnchor(anchor) && anchor.contains(target);
}
function isElementAnchor(anchor) {
    return isNode(anchor) &&
        anchor.nodeType === 1 &&
        typeof anchor.getBoundingClientRect === "function";
}
