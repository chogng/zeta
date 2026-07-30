import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IRectangle } from "../../browser/geometry.js";

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

const { Grid, SerializableGrid } = await import("../../browser/ui/grid/grid.js");

class TestGridView {
  readonly element: HTMLDivElement;
  readonly layouts: Array<{
    readonly width: number;
    readonly height: number;
    readonly top: number;
    readonly left: number;
  }> = [];
  visible = true;

  constructor(
    ownerDocument: Document,
    readonly minimumWidth: number,
    readonly maximumWidth: number,
    readonly minimumHeight = 0,
    readonly maximumHeight = Number.POSITIVE_INFINITY,
  ) {
    this.element = ownerDocument.createElement("div");
  }

  layout(bounds: IRectangle): void {
    this.layouts.push(bounds);
  }

  setVisible(visible: boolean): void {
    this.visible = visible;
  }
}

class SerializableTestGridView extends TestGridView {
  constructor(
    ownerDocument: Document,
    readonly id: string,
    minimumWidth: number,
    maximumWidth: number,
  ) {
    super(ownerDocument, minimumWidth, maximumWidth);
  }

  toJSON(): string {
    return this.id;
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
  assert.throws(
    () => new Grid({
      type: "branch",
      orientation: "horizontal",
      size: 200,
      children: [
        { type: "leaf", view, size: 100 },
        { type: "leaf", view, size: 100 },
      ],
    }, dom.window.document),
    /same view twice/,
  );
  dom.window.close();
});

test("SerializableGrid restores view identity and runtime geometry", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const left = new SerializableTestGridView(dom.window.document, "left", 100, 600);
  const editor = new SerializableTestGridView(dom.window.document, "editor", 120, Infinity);
  const right = new SerializableTestGridView(dom.window.document, "right", 100, 600);
  const grid = new SerializableGrid({
    type: "branch",
    orientation: "horizontal",
    size: 800,
    children: [
      { type: "leaf", view: left, size: 200 },
      { type: "leaf", view: editor, size: 400, priority: "high" },
      { type: "leaf", view: right, size: 200 },
    ],
  }, dom.window.document);
  grid.layout(900, 600);
  grid.resizeView(right, { width: 250, height: 600 });
  grid.setViewVisible(left, false);

  const snapshot = grid.serialize();
  const restoredViews = new Map<string, SerializableTestGridView>();
  const restored = SerializableGrid.deserialize(
    snapshot,
    {
      fromJSON: (data) => {
        if (typeof data !== "string") {
          throw new TypeError("Serialized test view ID must be a string");
        }
        const view = new SerializableTestGridView(
          dom.window.document,
          data,
          data === "editor" ? 120 : 100,
          data === "editor" ? Infinity : 600,
        );
        restoredViews.set(data, view);
        return view;
      },
    },
    dom.window.document,
  );
  restored.layout(900, 600);

  assert.equal(restored.isViewVisible(restoredViews.get("left")!), false);
  assert.equal(restored.getViewSize(restoredViews.get("editor")!).width, 650);
  assert.equal(restored.getViewSize(restoredViews.get("right")!).width, 250);

  restored.dispose();
  grid.dispose();
  dom.window.close();
});
