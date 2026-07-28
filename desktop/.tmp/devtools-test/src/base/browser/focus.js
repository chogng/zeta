import { Emitter } from "../common/event.js";
import { DisposableOwner, DisposableSlot, toDisposable, } from "../common/lifecycle.js";
import { addDisposableListener, isAncestor, isHTMLElement, isNode, } from "./dom.js";
import { getWindow, getWindows, isWindow, mainWindow, } from "./window.js";
const externalFocusCheckers = new Set();
const tabbableSelector = [
    "a[href]",
    "area[href]",
    "button",
    "input",
    "select",
    "textarea",
    "iframe",
    "object",
    "embed",
    "audio[controls]",
    "video[controls]",
    "summary",
    "[contenteditable]",
    "[tabindex]",
].join(",");
export var FocusNavigationDirection;
(function (FocusNavigationDirection) {
    FocusNavigationDirection[FocusNavigationDirection["Forward"] = 0] = "Forward";
    FocusNavigationDirection[FocusNavigationDirection["Backward"] = 1] = "Backward";
})(FocusNavigationDirection || (FocusNavigationDirection = {}));
export var FocusNavigationBoundary;
(function (FocusNavigationBoundary) {
    FocusNavigationBoundary[FocusNavigationBoundary["Stop"] = 0] = "Stop";
    FocusNavigationBoundary[FocusNavigationBoundary["Wrap"] = 1] = "Wrap";
})(FocusNavigationBoundary || (FocusNavigationBoundary = {}));
export function registerExternalFocusChecker(checker) {
    externalFocusCheckers.add(checker);
    return toDisposable(() => externalFocusCheckers.delete(checker));
}
export function getExternalFocusWindow() {
    for (const checker of externalFocusCheckers) {
        const result = checker();
        if (result.hasFocus)
            return result.window;
    }
    return undefined;
}
export function hasAppFocus() {
    return getWindows().some(({ window }) => window.document.hasFocus()) ||
        [...externalFocusCheckers].some((checker) => checker().hasFocus);
}
export function getActiveWindow() {
    return getWindows().find(({ window }) => window.document.hasFocus())?.window ??
        getExternalFocusWindow() ??
        mainWindow;
}
export function getActiveDocument() {
    return getActiveWindow().document;
}
/** Returns the deepest active element, including open shadow roots. */
export function getActiveElement(root = getActiveDocument()) {
    let active = root.activeElement;
    while (active?.shadowRoot?.activeElement) {
        active = active.shadowRoot.activeElement;
    }
    return active;
}
export function isActiveElement(element) {
    return getActiveElement(element.ownerDocument) === element;
}
export function isAncestorOfActiveElement(ancestor) {
    return isComposedAncestor(getActiveElement(ancestor.ownerDocument), ancestor);
}
export function trackFocus(target) {
    return new FocusTracker(target);
}
/** Focuses an element without changing the scroll positions of its ancestors. */
export function focusPreservingScroll(element) {
    const positions = saveAncestorScrollPositions(element);
    element.focus({ preventScroll: true });
    restoreAncestorScrollPositions(positions);
}
/** Restores focus only while the captured element remains connected. */
export function restoreFocus(element) {
    if (!element?.isConnected)
        return false;
    focusPreservingScroll(element);
    return isAncestorOfActiveElement(element);
}
/**
 * Returns whether an element can receive programmatic focus in its current
 * rendered and enabled state.
 */
export function isFocusable(element) {
    if (!element.isConnected || isUnavailableForFocus(element))
        return false;
    if (element.hasAttribute("tabindex"))
        return true;
    if (element.isContentEditable)
        return true;
    switch (element.tagName) {
        case "A":
        case "AREA":
            return element.hasAttribute("href");
        case "BUTTON":
        case "INPUT":
        case "SELECT":
        case "TEXTAREA":
        case "IFRAME":
        case "OBJECT":
        case "EMBED":
        case "SUMMARY":
            return true;
        case "AUDIO":
        case "VIDEO":
            return element.hasAttribute("controls");
        default:
            return false;
    }
}
/** Returns whether an element participates in sequential Tab navigation. */
export function isTabbable(element) {
    return isFocusable(element) &&
        element.tabIndex >= 0 &&
        isTabbableRadio(element);
}
/**
 * Returns tabbable descendants in browser navigation order: positive
 * `tabindex` values first, then normal DOM order.
 */
export function getTabbableElements(container) {
    return Array.from(container.querySelectorAll(tabbableSelector))
        .filter(isTabbable)
        .map((element, order) => ({ element, order }))
        .sort((left, right) => {
        const leftIndex = left.element.tabIndex;
        const rightIndex = right.element.tabIndex;
        if (leftIndex > 0 && rightIndex <= 0)
            return -1;
        if (leftIndex <= 0 && rightIndex > 0)
            return 1;
        if (leftIndex > 0 && rightIndex > 0 && leftIndex !== rightIndex) {
            return leftIndex - rightIndex;
        }
        return left.order - right.order;
    })
        .map(({ element }) => element);
}
export function focusFirst(container) {
    const first = getTabbableElements(container)[0];
    if (first)
        focusPreservingScroll(first);
    return first;
}
export function focusLast(container) {
    const elements = getTabbableElements(container);
    const last = elements[elements.length - 1];
    if (last)
        focusPreservingScroll(last);
    return last;
}
/**
 * Moves focus through the tabbable descendants of a container.
 *
 * `boundary` makes wrapping explicit at call sites instead of encoding it as
 * an ambiguous boolean argument.
 */
export function moveFocus(container, direction, boundary = FocusNavigationBoundary.Stop) {
    const elements = getTabbableElements(container);
    if (elements.length === 0)
        return undefined;
    const ownerDocument = getOwnerDocument(container);
    const activeElement = getActiveElement(ownerDocument);
    const currentIndex = elements.findIndex((element) => element === activeElement);
    let nextIndex = direction === FocusNavigationDirection.Forward
        ? currentIndex + 1
        : currentIndex < 0 ? elements.length - 1 : currentIndex - 1;
    if (nextIndex < 0 || nextIndex >= elements.length) {
        if (boundary === FocusNavigationBoundary.Stop)
            return undefined;
        nextIndex = nextIndex < 0 ? elements.length - 1 : 0;
    }
    const next = elements[nextIndex];
    if (next)
        focusPreservingScroll(next);
    return next;
}
/** Keeps sequential Tab navigation inside a container until disposed. */
export function trapTabFocus(container) {
    return addDisposableListener(container, "keydown", (event) => {
        if (event.isComposing ||
            event.key !== "Tab" ||
            event.altKey ||
            event.ctrlKey ||
            event.metaKey) {
            return;
        }
        const direction = event.shiftKey
            ? FocusNavigationDirection.Backward
            : FocusNavigationDirection.Forward;
        const next = moveFocus(container, direction, FocusNavigationBoundary.Wrap);
        if (next || isAncestorOfActiveElement(container)) {
            event.preventDefault();
        }
    }, true);
}
export function saveAncestorScrollPositions(node) {
    const positions = [];
    for (let current = composedParent(node); current; current = composedParent(current)) {
        positions.push({
            element: current,
            left: current.scrollLeft,
            top: current.scrollTop,
        });
    }
    return positions;
}
export function restoreAncestorScrollPositions(positions) {
    for (const position of positions) {
        position.element.scrollTo(position.left, position.top);
    }
}
class FocusTracker extends DisposableOwner {
    #onDidFocus = this.own(new Emitter());
    #onDidBlur = this.own(new Emitter());
    #pendingRefresh = this.own(new DisposableSlot());
    onDidFocus = this.#onDidFocus.event;
    onDidBlur = this.#onDidBlur.event;
    #target;
    #hasFocus;
    constructor(target) {
        super();
        this.#target = target;
        if (isWindow(target)) {
            this.#hasFocus = target.document.hasFocus();
            this.own(addDisposableListener(target, "focus", () => this.#setFocused(true)));
            this.own(addDisposableListener(target, "blur", () => this.#setFocused(false)));
            return;
        }
        this.#hasFocus = isAncestorOfActiveElement(target);
        this.own(addDisposableListener(target, "focusin", () => {
            this.#pendingRefresh.clear();
            this.#setFocused(true);
        }));
        this.own(addDisposableListener(target, "focusout", () => this.#scheduleRefresh()));
        this.own(addDisposableListener(getWindow(target), "blur", () => this.#setFocused(false)));
    }
    get hasFocus() {
        return this.#hasFocus;
    }
    refreshState() {
        this.#pendingRefresh.clear();
        this.#setFocused(isWindow(this.#target)
            ? this.#target.document.hasFocus()
            : isAncestorOfActiveElement(this.#target));
    }
    #scheduleRefresh() {
        if (this.#pendingRefresh.value)
            return;
        const targetWindow = getWindow(this.#target);
        const handle = targetWindow.setTimeout(() => {
            this.#pendingRefresh.clear();
            this.refreshState();
        }, 0);
        this.#pendingRefresh.replace(toDisposable(() => targetWindow.clearTimeout(handle)));
    }
    #setFocused(focused) {
        if (focused === this.#hasFocus)
            return;
        this.#hasFocus = focused;
        if (focused)
            this.#onDidFocus.fire();
        else
            this.#onDidBlur.fire();
    }
}
function isUnavailableForFocus(element) {
    if (element.hidden ||
        element.closest("[inert], [aria-hidden='true']") !== null ||
        element.matches(":disabled")) {
        return true;
    }
    const style = getWindow(element).getComputedStyle(element);
    return style.display === "none" ||
        style.visibility === "hidden" ||
        style.visibility === "collapse" ||
        element.getClientRects().length === 0;
}
function getOwnerDocument(node) {
    if (isNode(node)) {
        return node.nodeType === 9
            ? node
            : node.ownerDocument ?? getActiveDocument();
    }
    return getActiveDocument();
}
function composedParent(element) {
    if (element.parentElement)
        return element.parentElement;
    const root = element.getRootNode();
    return isShadowRoot(root) ? root.host : null;
}
function isComposedAncestor(candidate, ancestor) {
    for (let current = candidate; current; current = composedParent(current)) {
        if (current === ancestor || isAncestor(current, ancestor))
            return true;
    }
    return false;
}
function isShadowRoot(node) {
    return node.nodeType === 11 &&
        "host" in node &&
        isHTMLElement(node.host);
}
function isTabbableRadio(element) {
    if (element.tagName !== "INPUT" ||
        element.type !== "radio") {
        return true;
    }
    const radio = element;
    if (!radio.name)
        return true;
    const root = radio.form ??
        radio.getRootNode();
    const group = Array.from(root.querySelectorAll("input[type='radio']")).filter((candidate) => candidate.name === radio.name &&
        candidate.form === radio.form &&
        isFocusable(candidate));
    return (group.find((candidate) => candidate.checked) ?? group[0]) === radio;
}
