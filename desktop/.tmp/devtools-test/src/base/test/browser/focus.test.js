import assert from "node:assert/strict";
import test from "node:test";
class FakeNode extends EventTarget {
    static ELEMENT_NODE = 1;
    static DOCUMENT_NODE = 9;
}
class FakeDocument extends EventTarget {
    nodeType = FakeNode.DOCUMENT_NODE;
    defaultView;
    activeElement = null;
    hasFocus() {
        return this.activeElement !== null;
    }
}
class FakeWindow extends EventTarget {
    document;
    window = this;
    constructor(document) {
        super();
        this.document = document;
    }
    setTimeout = globalThis.setTimeout.bind(globalThis);
    clearTimeout = globalThis.clearTimeout.bind(globalThis);
    getComputedStyle() {
        return { display: "block", visibility: "visible" };
    }
}
class FakeElement extends FakeNode {
    ownerDocument;
    tagName;
    tabIndex;
    disabled;
    rendered;
    nodeType = FakeNode.ELEMENT_NODE;
    namespaceURI = "http://www.w3.org/1999/xhtml";
    parentElement = null;
    shadowRoot = null;
    isConnected = true;
    hidden = false;
    isContentEditable = false;
    scrollLeft = 0;
    scrollTop = 0;
    #attributes = new Map();
    #children = [];
    constructor(ownerDocument, tagName, tabIndex, disabled = false, rendered = true) {
        super();
        this.ownerDocument = ownerDocument;
        this.tagName = tagName;
        this.tabIndex = tabIndex;
        this.disabled = disabled;
        this.rendered = rendered;
    }
    setChildren(children) {
        this.#children = [...children];
    }
    hasAttribute(name) {
        return this.#attributes.has(name);
    }
    getAttribute(name) {
        return this.#attributes.get(name) ?? null;
    }
    closest() {
        return null;
    }
    matches(selector) {
        return selector === ":disabled" && this.disabled;
    }
    getClientRects() {
        return this.rendered ? [{}] : [];
    }
    getRootNode() {
        return this.ownerDocument;
    }
    querySelectorAll() {
        return this.#children;
    }
    contains(candidate) {
        const element = candidate;
        return element === this ||
            this.#children.includes(element);
    }
    focus() {
        this.ownerDocument.activeElement = this;
    }
    scrollTo() { }
}
Object.defineProperty(globalThis, "Node", {
    configurable: true,
    value: FakeNode,
});
const fakeDocument = new FakeDocument();
const fakeWindow = new FakeWindow(fakeDocument);
fakeDocument.defaultView = fakeWindow;
Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: fakeWindow,
});
const { FocusNavigationBoundary, FocusNavigationDirection, getTabbableElements, moveFocus, } = await import("../../browser/focus.js");
test("tabbable elements follow positive tabindex then DOM order", () => {
    const normal = new FakeElement(fakeDocument, "BUTTON", 0);
    const second = new FakeElement(fakeDocument, "BUTTON", 2);
    const first = new FakeElement(fakeDocument, "BUTTON", 1);
    const disabled = new FakeElement(fakeDocument, "BUTTON", 0, true);
    const hidden = new FakeElement(fakeDocument, "BUTTON", 0, false, false);
    const container = new FakeElement(fakeDocument, "DIV", -1);
    container.setChildren([normal, second, first, disabled, hidden]);
    assert.deepEqual(getTabbableElements(container), [first, second, normal]);
});
test("moveFocus wraps explicitly at the container boundary", () => {
    const first = new FakeElement(fakeDocument, "BUTTON", 1);
    const second = new FakeElement(fakeDocument, "BUTTON", 2);
    const normal = new FakeElement(fakeDocument, "BUTTON", 0);
    const container = new FakeElement(fakeDocument, "DIV", -1);
    container.setChildren([normal, second, first]);
    normal.focus();
    const focused = moveFocus(container, FocusNavigationDirection.Forward, FocusNavigationBoundary.Wrap);
    assert.equal(focused, first);
    assert.equal(fakeDocument.activeElement, first);
});
