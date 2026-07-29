import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const environment = new JSDOM(
  "<!doctype html><html><head></head><body><main></main></body></html>",
);
for (const [name, value] of Object.entries({
  window: environment.window,
  document: environment.window.document,
  Node: environment.window.Node,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const {
  BrowserContextViewService,
} = await import(
  "../src/zeta/platform/contextview/browser/contextViewService.js"
);

test("context view service hosts overlays inside its Workbench container", () => {
  const container = environment.window.document.querySelector("main");
  assert.ok(container);
  const service = new BrowserContextViewService(container);
  const content = environment.window.document.createElement("div");
  content.textContent = "Menu";

  assert.equal(
    service.show({
      anchor: { left: 8, top: 12, width: 0, height: 0 },
      content,
    }),
    true,
  );

  const contextView = container.querySelector(":scope > .zeta-context-view");
  assert.ok(contextView);
  assert.equal(contextView.contains(content), true);

  service.hide();
  assert.equal(contextView.hasAttribute("hidden"), true);
  service.dispose();
  assert.equal(container.querySelector(".zeta-context-view"), null);
});
