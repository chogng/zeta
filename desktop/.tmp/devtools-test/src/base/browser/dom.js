import { toDisposable, } from "../common/lifecycle.js";
export function addDisposableListener(target, type, listener, options) {
    const eventListener = listener;
    target.addEventListener(type, eventListener, options);
    let activeTarget = target;
    let activeListener = eventListener;
    return toDisposable(() => {
        activeTarget?.removeEventListener(type, activeListener, options);
        activeTarget = undefined;
        activeListener = undefined;
    });
}
/** Removes every child from a DOM container. */
export function clearNode(node) {
    node.replaceChildren();
    return node;
}
export function append(parent, ...children) {
    parent.append(...children);
    return children.length === 1 && typeof children[0] !== "string"
        ? children[0]
        : undefined;
}
/** Replaces a container's children with the supplied nodes or text. */
export function reset(parent, ...children) {
    parent.replaceChildren(...children);
}
/** Updates native visibility without overwriting layout-related styles. */
export function setVisibility(visible, ...elements) {
    for (const element of elements)
        element.hidden = !visible;
}
export function show(...elements) {
    setVisibility(true, ...elements);
}
export function hide(...elements) {
    setVisibility(false, ...elements);
}
/** Tests DOM ancestry without assuming that either node is connected. */
export function isAncestor(candidate, ancestor) {
    return Boolean(candidate && ancestor?.contains(candidate));
}
export function isNode(value) {
    return typeof value === "object" &&
        value !== null &&
        typeof value.nodeType === "number";
}
/** Cross-realm HTMLElement guard that does not rely on global instanceof. */
export function isHTMLElement(value) {
    return isNode(value) &&
        value.nodeType === 1 &&
        value.namespaceURI === "http://www.w3.org/1999/xhtml";
}
export function isHTMLInputElement(value) {
    return isHTMLElement(value) && value.tagName === "INPUT";
}
export function isHTMLButtonElement(value) {
    return isHTMLElement(value) && value.tagName === "BUTTON";
}
/** Stops propagation and, by default, the browser's native behavior. */
export function stopEvent(event, options = {}) {
    if (options.preventDefault !== false)
        event.preventDefault();
    if (options.immediate)
        event.stopImmediatePropagation();
    else
        event.stopPropagation();
}
