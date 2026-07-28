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
const { Grid } = await import("../../browser/ui/grid/grid.js");
class TestGridView {
    minimumWidth;
    maximumWidth;
    minimumHeight;
    maximumHeight;
    element;
    layouts = [];
    visible = true;
    constructor(ownerDocument, minimumWidth, maximumWidth, minimumHeight = 0, maximumHeight = Number.POSITIVE_INFINITY) {
        this.minimumWidth = minimumWidth;
        this.maximumWidth = maximumWidth;
        this.minimumHeight = minimumHeight;
        this.maximumHeight = maximumHeight;
        this.element = ownerDocument.createElement("div");
    }
    layout(bounds) {
        this.layouts.push(bounds);
    }
    setVisible(visible) {
        this.visible = visible;
    }
}
test("Grid lays out a nested SplitView tree in two dimensions", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const left = new TestGridView(dom.window.document, 100, 600);
    const session = new TestGridView(dom.window.document, 120, Infinity, 36, 36);
    const editor = new TestGridView(dom.window.document, 120, Infinity, 84);
    const right = new TestGridView(dom.window.document, 100, 600);
    const grid = new Grid({
        type: "branch",
        orientation: "horizontal",
        size: 800,
        children: [
            { type: "leaf", view: left, size: 200 },
            {
                type: "branch",
                orientation: "vertical",
                size: 400,
                children: [
                    { type: "leaf", view: session, size: 36 },
                    { type: "leaf", view: editor, size: 564 },
                ],
            },
            { type: "leaf", view: right, size: 200 },
        ],
    }, dom.window.document);
    grid.layout(800, 600);
    assert.deepEqual(left.layouts.at(-1), {
        width: 200,
        height: 600,
        top: 0,
        left: 0,
    });
    assert.deepEqual(session.layouts.at(-1), {
        width: 400,
        height: 36,
        top: 0,
        left: 200,
    });
    assert.deepEqual(editor.layouts.at(-1), {
        width: 400,
        height: 564,
        top: 36,
        left: 200,
    });
    assert.equal(grid.element.querySelectorAll(".zeta-sash").length, 2);
    grid.dispose();
    dom.window.close();
});
test("Grid keeps hidden leaves mounted and restores their pixel size", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const left = new TestGridView(dom.window.document, 100, 600);
    const editor = new TestGridView(dom.window.document, 120, Infinity);
    const right = new TestGridView(dom.window.document, 100, 600);
    const grid = new Grid({
        type: "branch",
        orientation: "horizontal",
        size: 800,
        children: [
            { type: "leaf", view: left, size: 200 },
            { type: "leaf", view: editor, size: 400 },
            { type: "leaf", view: right, size: 200 },
        ],
    }, dom.window.document);
    grid.layout(800, 600);
    grid.setViewVisible(left, false);
    assert.equal(left.visible, false);
    assert.ok(left.element.parentElement);
    assert.deepEqual(grid.getViewSize(editor), { width: 500, height: 600 });
    grid.setViewVisible(left, true);
    assert.equal(left.visible, true);
    assert.deepEqual(grid.getViewSize(left), { width: 200, height: 600 });
    assert.deepEqual(grid.getViewSize(editor), { width: 400, height: 600 });
    grid.resizeView(left, { width: 250, height: 600 });
    assert.deepEqual(grid.getViewSize(left), { width: 250, height: 600 });
    grid.dispose();
    dom.window.close();
});
test("Grid rejects duplicate views in its descriptor", () => {
    const dom = new JSDOM("<!doctype html><body></body>");
    const view = new TestGridView(dom.window.document, 0, Infinity);
    assert.throws(() => new Grid({
        type: "branch",
        orientation: "horizontal",
        size: 200,
        children: [
            { type: "leaf", view, size: 100 },
            { type: "leaf", view, size: 100 },
        ],
    }, dom.window.document), /same view twice/);
    dom.window.close();
});
