import { getWindow } from "./window.js";
export class Dimension {
    width;
    height;
    static Zero = new Dimension(0, 0);
    constructor(width, height) {
        this.width = width;
        this.height = height;
    }
    with(width = this.width, height = this.height) {
        return width === this.width && height === this.height
            ? this
            : new Dimension(width, height);
    }
    static equals(left, right) {
        return left === right ||
            Boolean(left && right &&
                left.width === right.width &&
                left.height === right.height);
    }
}
export function getClientArea(element) {
    const targetWindow = getWindow(element);
    if (element === targetWindow.document.body) {
        const viewport = targetWindow.visualViewport;
        return new Dimension(viewport?.width ?? targetWindow.innerWidth, viewport?.height ?? targetWindow.innerHeight);
    }
    return new Dimension(element.clientWidth, element.clientHeight);
}
export function getViewport(targetWindow) {
    const viewport = targetWindow.visualViewport;
    return {
        left: viewport?.offsetLeft ?? 0,
        top: viewport?.offsetTop ?? 0,
        width: viewport?.width ?? targetWindow.innerWidth,
        height: viewport?.height ?? targetWindow.innerHeight,
    };
}
export function getDomNodePagePosition(element) {
    const bounds = element.getBoundingClientRect();
    const targetWindow = getWindow(element);
    return {
        left: bounds.left + targetWindow.scrollX,
        top: bounds.top + targetWindow.scrollY,
        width: bounds.width,
        height: bounds.height,
    };
}
export function getContentSize(element) {
    const style = getWindow(element).getComputedStyle(element);
    return new Dimension(element.offsetWidth -
        pixels(style.borderLeftWidth) -
        pixels(style.borderRightWidth) -
        pixels(style.paddingLeft) -
        pixels(style.paddingRight), element.offsetHeight -
        pixels(style.borderTopWidth) -
        pixels(style.borderBottomWidth) -
        pixels(style.paddingTop) -
        pixels(style.paddingBottom));
}
export function getTotalSize(element) {
    const style = getWindow(element).getComputedStyle(element);
    return new Dimension(element.offsetWidth +
        pixels(style.marginLeft) +
        pixels(style.marginRight), element.offsetHeight +
        pixels(style.marginTop) +
        pixels(style.marginBottom));
}
function pixels(value) {
    return Number.parseFloat(value) || 0;
}
