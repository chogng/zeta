import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IAction } from "../../common/actions.js";
import { Separator } from "../../common/actions.js";
import { ActionBar } from "../../browser/ui/actionbar/actionbar.js";

test("ActionBar owns horizontal keyboard navigation", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const actionBar = new ActionBar({
    ownerDocument: dom.window.document,
    actions: [
      action("first"),
      new Separator(),
      action("disabled", false),
      action("last"),
    ],
  });
  dom.window.document.body.append(actionBar.element);
  const buttons = actionBar.element.querySelectorAll("button");
  const first = buttons[0];
  const disabled = buttons[1];
  const last = buttons[2];
  assert.ok(first);
  assert.ok(disabled);
  assert.ok(last);
  assert.equal(actionBar.element.getAttribute("role"), "toolbar");
  assert.equal(actionBar.element.getAttribute("aria-orientation"), "horizontal");
  assert.deepEqual(
    [...buttons].map((button) => button.tabIndex),
    [0, -1, -1],
  );

  first.focus();
  first.dispatchEvent(keyboardEvent(dom.window, "ArrowRight"));
  assert.equal(dom.window.document.activeElement, last);
  assert.deepEqual(
    [...buttons].map((button) => button.tabIndex),
    [-1, -1, 0],
  );
  last.dispatchEvent(keyboardEvent(dom.window, "ArrowRight"));
  assert.equal(dom.window.document.activeElement, first);
  first.dispatchEvent(keyboardEvent(dom.window, "ArrowLeft"));
  assert.equal(dom.window.document.activeElement, last);
  last.dispatchEvent(keyboardEvent(dom.window, "Home"));
  assert.equal(dom.window.document.activeElement, first);
  first.dispatchEvent(keyboardEvent(dom.window, "End"));
  assert.equal(dom.window.document.activeElement, last);

  actionBar.dispose();
  dom.window.close();
});

test("ActionBar maps vertical navigation to up and down", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const actionBar = new ActionBar({
    ownerDocument: dom.window.document,
    actions: [action("first"), action("second")],
    orientation: "vertical",
  });
  dom.window.document.body.append(actionBar.element);
  const buttons = actionBar.element.querySelectorAll("button");
  const first = buttons[0];
  const second = buttons[1];
  assert.ok(first);
  assert.ok(second);
  assert.equal(actionBar.element.getAttribute("role"), "toolbar");
  assert.equal(actionBar.element.getAttribute("aria-orientation"), "vertical");

  first.focus();
  first.dispatchEvent(keyboardEvent(dom.window, "ArrowDown"));
  assert.equal(dom.window.document.activeElement, second);
  second.dispatchEvent(keyboardEvent(dom.window, "ArrowUp"));
  assert.equal(dom.window.document.activeElement, first);
  first.dispatchEvent(keyboardEvent(dom.window, "ArrowRight"));
  assert.equal(dom.window.document.activeElement, first);

  actionBar.dispose();
  dom.window.close();
});

function action(
  id: string,
  enabled = true,
): IAction {
  return {
    id,
    label: id,
    tooltip: id,
    enabled,
    run() {},
  };
}

function keyboardEvent(
  targetWindow: { readonly KeyboardEvent: typeof KeyboardEvent },
  key: string,
): KeyboardEvent {
  return new targetWindow.KeyboardEvent("keydown", {
    bubbles: true,
    cancelable: true,
    key,
  });
}
