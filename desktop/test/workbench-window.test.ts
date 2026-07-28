import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";

const mainEnvironment = new JSDOM(
  "<!doctype html><html><head></head><body></body></html>",
);
for (const [name, value] of Object.entries({
  window: mainEnvironment.window,
  document: mainEnvironment.window.document,
  Node: mainEnvironment.window.Node,
  MutationObserver: mainEnvironment.window.MutationObserver,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const {
  isRegisteredWindow,
} = await import("../src/zeta/base/browser/window.js");
const {
  WorkbenchState,
} = await import(
  "../src/zeta/platform/workspace/common/workspace.js"
);
const {
  WorkbenchWindow,
} = await import("../src/zeta/workbench/browser/window.js");

test(
  "WorkbenchWindow owns root identity and secondary document integration",
  async () => {
    const mainStyle =
      mainEnvironment.window.document.createElement("style");
    mainStyle.dataset.source = "main";
    mainStyle.textContent = ".from-main { color: red; }";
    mainEnvironment.window.document.head.append(mainStyle);

    const secondaryEnvironment = new JSDOM(
      "<!doctype html><html><head></head><body>" +
        "<main><span></span></main></body></html>",
    );
    const secondaryWindow =
      secondaryEnvironment.window as unknown as Window;
    const root =
      secondaryEnvironment.window.document.querySelector("main");
    assert.ok(root);

    const workbenchWindow = new WorkbenchWindow({
      root,
      productId: "academic",
      workbenchState: WorkbenchState.FOLDER,
    });

    assert.equal(root.classList.contains("zeta-workbench"), true);
    assert.equal(root.dataset.product, "academic");
    assert.equal(root.dataset.workbenchState, "folder");
    assert.equal(
      isRegisteredWindow(secondaryWindow),
      true,
    );
    assert.equal(
      secondaryEnvironment.window.document.head
        .querySelector("style[data-source='main']")
        ?.textContent,
      mainStyle.textContent,
    );

    const nextStyle =
      mainEnvironment.window.document.createElement("style");
    nextStyle.dataset.source = "next";
    mainEnvironment.window.document.head.append(nextStyle);
    await new Promise((resolve) => globalThis.setTimeout(resolve, 0));
    assert.ok(
      secondaryEnvironment.window.document.head
        .querySelector("style[data-source='next']"),
    );

    workbenchWindow.dispose();

    assert.equal(root.classList.contains("zeta-workbench"), false);
    assert.equal(root.hasAttribute("data-product"), false);
    assert.equal(root.childElementCount, 0);
    assert.equal(
      isRegisteredWindow(secondaryWindow),
      false,
    );
    assert.equal(
      secondaryEnvironment.window.document.head.childElementCount,
      0,
    );
  },
);
