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
  MouseEvent: browserEnvironment.window.MouseEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { TerminalTabsLayout } = await import("../../../../../workbench/contrib/terminal/browser/view/terminalTabsLayout.js");

test.after(() => {
  browserEnvironment.window.close();
  for (const name of ["window", "document", "Node", "Element", "HTMLElement", "Event", "KeyboardEvent", "MouseEvent"]) {
    Reflect.deleteProperty(globalThis, name);
  }
});

test("Terminal instance list sash resizes the right column within its bounds", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const widgets = dom.window.document.createElement("main");
  const tabs = dom.window.document.createElement("aside");
  using layout = new TerminalTabsLayout(widgets, tabs);
  dom.window.document.body.append(layout.element);
  layout.layout(1_000, 200);
  const panes = layout.element.querySelectorAll<HTMLElement>(":scope > .zeta-split-view-pane");
  const sash = layout.element.querySelector<HTMLElement>(":scope > .zeta-sash");
  assert.equal(panes.length, 2);
  assert.ok(sash);
  assert.equal(panes[0]?.style.width, "880px");
  assert.equal(panes[1]?.style.width, "120px");
  assert.equal(sash.getAttribute("aria-label"), "Resize terminal instance list");

  sash.dispatchEvent(new dom.window.MouseEvent("pointerdown", { button: 0, clientX: 880, bubbles: true }));
  dom.window.dispatchEvent(new dom.window.MouseEvent("pointermove", { clientX: 950, bubbles: true }));
  assert.equal(panes[1]?.style.width, "46px");
  assert.equal(tabs.classList.contains("zeta-terminal-tabs-narrow"), true);
  dom.window.dispatchEvent(new dom.window.MouseEvent("pointerup", { clientX: 950, bubbles: true }));

  sash.dispatchEvent(new dom.window.MouseEvent("pointerdown", { button: 0, clientX: 954, bubbles: true }));
  dom.window.dispatchEvent(new dom.window.MouseEvent("pointermove", { clientX: 930, bubbles: true }));
  assert.equal(panes[1]?.style.width, "80px");
  assert.equal(tabs.classList.contains("zeta-terminal-tabs-narrow"), false);
  dom.window.dispatchEvent(new dom.window.MouseEvent("pointerup", { clientX: 930, bubbles: true }));

  for (let index = 0; index < 50; index += 1) {
    sash.dispatchEvent(new dom.window.KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }));
  }
  assert.equal(panes[1]?.style.width, "500px");

  layout.setInstanceListPresentation("hidden");
  assert.equal(panes[1]?.hidden, true);
  assert.equal(panes[0]?.style.width, "1000px");
  assert.equal(layout.element.querySelector(":scope > .zeta-sash"), null);

  layout.setInstanceListPresentation("visible");
  assert.equal(panes[1]?.hidden, false);
  assert.equal(panes[1]?.style.width, "500px");
  assert.equal(layout.element.querySelector(":scope > .zeta-sash")?.getAttribute("aria-label"), "Resize terminal instance list");
  dom.window.close();
});
