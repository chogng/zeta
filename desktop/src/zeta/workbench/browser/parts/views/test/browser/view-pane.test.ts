import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IViewPaneOptions } from "../../../../../../workbench/browser/parts/views/viewPane.js";

test("ViewPane title chevron tracks collapsed state", async () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const installedGlobals = installDomGlobals(dom);
  try {
    const { ViewPane } = await import("../../../../../../workbench/browser/parts/views/viewPane.js");
    class TestViewPane extends ViewPane {
      constructor(options: IViewPaneOptions) {
        super(options);
      }
    }
    using pane = new TestViewPane({
      id: "test.pane",
      title: "Test Pane",
      ownerDocument: dom.window.document,
      collapsed: true,
    });
    dom.window.document.body.append(pane.element);

    const title = pane.element.querySelector(".zeta-view-pane-title");
    const button = pane.element.querySelector<HTMLButtonElement>(".zeta-view-pane-title-button");
    const content = pane.element.querySelector<HTMLElement>(".zeta-view-pane-content");
    assert.equal(title?.textContent, "Test Pane");
    assert.equal(button?.getAttribute("aria-expanded"), "false");
    assert.equal(button?.getAttribute("aria-controls"), content?.id);
    assert.equal(button?.classList.contains("expanded"), false);
    assert.equal(pane.element.classList.contains("collapsed"), true);
    assert.equal(content?.hidden, true);
    assert.equal(button?.querySelectorAll(".zeta-icon").length, 2);

    button?.click();
    assert.equal(pane.isCollapsed(), false);
    assert.equal(button?.getAttribute("aria-expanded"), "true");
    assert.equal(button?.classList.contains("expanded"), true);
    assert.equal(pane.element.classList.contains("collapsed"), false);
    assert.equal(content?.hidden, false);

    pane.setTitle("Renamed Pane");
    assert.equal(title?.textContent, "Renamed Pane");
    assert.equal(button?.querySelectorAll(".zeta-icon").length, 2);
  } finally {
    for (const name of installedGlobals) Reflect.deleteProperty(globalThis, name);
    dom.window.close();
  }
});

function installDomGlobals(dom: JSDOM): readonly string[] {
  const globals = {
    window: dom.window,
    document: dom.window.document,
    Node: dom.window.Node,
    Element: dom.window.Element,
    HTMLElement: dom.window.HTMLElement,
    Event: dom.window.Event,
    MouseEvent: dom.window.MouseEvent,
    navigator: dom.window.navigator,
  };
  for (const [name, value] of Object.entries(globals)) {
    Object.defineProperty(globalThis, name, {
      configurable: true,
      value,
    });
  }
  return Object.keys(globals);
}
