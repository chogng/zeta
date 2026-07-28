import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
const browserEnvironment = new JSDOM("<!doctype html><body></body>");
for (const [name, value] of Object.entries({
    window: browserEnvironment.window,
    document: browserEnvironment.window.document,
    Node: browserEnvironment.window.Node,
    Element: browserEnvironment.window.Element,
    HTMLElement: browserEnvironment.window.HTMLElement,
    Event: browserEnvironment.window.Event,
    KeyboardEvent: browserEnvironment.window.KeyboardEvent,
    PointerEvent: browserEnvironment.window.MouseEvent,
})) {
    Object.defineProperty(globalThis, name, {
        configurable: true,
        value,
    });
}
const { SplitView } = await import("../../browser/ui/splitview/splitview.js");
class TestView {
    minimumSize;
    maximumSize;
    element;
    layouts = [];
    visible = true;
    constructor(ownerDocument, minimumSize, maximumSize) {
        this.minimumSize = minimumSize;
        this.maximumSize = maximumSize;
        this.element = ownerDocument.createElement("div");
    }
    layout(size, offset, orthogonalSize) {
        this.layouts.push({ size, offset, orthogonalSize });
    }
    setVisible(visible) {
        this.visible = visible;
    }
}
test("SplitView owns constrained pixel sizes and positions", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const split = new SplitView("horizontal", dom.window.document);
    const fixed = new TestView(dom.window.document, 35, 35);
    const center = new TestView(dom.window.document, 100, Infinity);
    const side = new TestView(dom.window.document, 180, 600);
    split.addView(fixed, 35);
    split.addView(center, 100);
    split.addView(side, 220);
    split.layout(500, 300);
    assert.equal(split.viewCount, 3);
    assert.deepEqual([0, 1, 2].map((index) => split.getViewSize(index)), [35, 172.5, 292.5]);
    assert.equal(split.element.querySelectorAll(".zeta-sash").length, 1);
    const panes = split.element.querySelectorAll(".zeta-split-view-pane");
    assert.equal(panes[0]?.style.left, "0px");
    assert.equal(panes[1]?.style.left, "35px");
    assert.equal(panes[2]?.style.left, "207.5px");
    assert.equal(panes[2]?.style.width, "292.5px");
    assert.deepEqual(side.layouts.at(-1), {
        size: 292.5,
        offset: 207.5,
        orthogonalSize: 300,
    });
    split.dispose();
    dom.window.close();
});
test("SplitView clamps absolute sash drag deltas without boundary jumps", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const split = new SplitView("horizontal", dom.window.document);
    split.addView(new TestView(dom.window.document, 100, 260), 200);
    split.addView(new TestView(dom.window.document, 100, Infinity), 200);
    split.layout(400, 100);
    const sash = split.element.querySelector(".zeta-sash");
    assert.ok(sash);
    sash.dispatchEvent(pointerEvent(dom.window, "pointerdown", 0));
    dom.window.dispatchEvent(pointerEvent(dom.window, "pointermove", 150));
    dom.window.dispatchEvent(pointerEvent(dom.window, "pointermove", 140));
    dom.window.dispatchEvent(pointerEvent(dom.window, "pointerup", 140));
    assert.deepEqual([split.getViewSize(0), split.getViewSize(1)], [260, 140]);
    split.dispose();
    dom.window.close();
});
test("SplitView retains and restores hidden view size", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const split = new SplitView("horizontal", dom.window.document);
    const sidebar = new TestView(dom.window.document, 180, 600);
    const editor = new TestView(dom.window.document, 120, Infinity);
    split.addView(sidebar, 220);
    split.addView(editor, 580);
    split.layout(800, 600);
    split.setViewVisible(0, false);
    assert.equal(split.getViewCachedVisibleSize(0), 220);
    assert.equal(split.getViewSize(1), 800);
    assert.equal(sidebar.visible, false);
    assert.equal(sidebar.element.parentElement?.parentElement, split.element);
    split.setViewVisible(0, true);
    assert.equal(split.getViewSize(0), 220);
    assert.equal(split.getViewSize(1), 580);
    assert.equal(sidebar.visible, true);
    split.dispose();
    dom.window.close();
});
test("SplitView sashes support keyboard resizing", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const split = new SplitView("horizontal", dom.window.document);
    split.addView(new TestView(dom.window.document, 40, Infinity), 100);
    split.addView(new TestView(dom.window.document, 40, Infinity), 100);
    split.layout(200, 100);
    const sash = split.element.querySelector(".zeta-sash");
    assert.ok(sash);
    sash.dispatchEvent(new dom.window.KeyboardEvent("keydown", {
        bubbles: true,
        key: "ArrowRight",
    }));
    assert.deepEqual([split.getViewSize(0), split.getViewSize(1)], [110, 90]);
    split.dispose();
    dom.window.close();
});
test("SplitView validates view constraints before mounting", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const split = new SplitView("horizontal", dom.window.document);
    assert.throws(() => split.addView(new TestView(dom.window.document, 100, 40), 100), /maximum size/);
    assert.equal(split.viewCount, 0);
    split.dispose();
    dom.window.close();
});
function pointerEvent(targetWindow, type, clientX) {
    return new targetWindow.MouseEvent(type, {
        bubbles: true,
        button: 0,
        clientX,
    });
}
