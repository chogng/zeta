import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import type { IAction } from "../../common/actions.js";
import { Separator } from "../../common/actions.js";
import { lxiconsLibrary } from "../../common/lxiconsLibrary.js";
import { ActionBar } from "../../browser/ui/actionbar/actionbar.js";
import { LabelActionViewItem } from "../../browser/ui/actionbar/actionViewItems.js";
import { setHoverDelegate, type HoverDelegateSetupOptions, type IManagedHover } from "../../browser/ui/hover/hoverDelegate.js";

test("ActionViewItem routes its tooltip through the shared action Hover group", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const setups: HoverDelegateSetupOptions[] = [];
  using delegateRegistration = setHoverDelegate({
    setupHover(options) {
      setups.push(options);
      options.target.removeAttribute("title");
      return managedHover();
    },
  });
  using actionBar = new ActionBar({
    ownerDocument: dom.window.document,
    actions: [action("open")],
  });
  dom.window.document.body.append(actionBar.element);

  const button = actionBar.element.querySelector("button");
  assert.ok(button);
  assert.equal(setups.length, 1);
  assert.equal(setups[0]?.target, button);
  assert.equal(setups[0]?.content, "open");
  assert.equal(setups[0]?.groupId, "actions");
  assert.equal(button.hasAttribute("title"), false);

  dom.window.close();
});

test("LabelActionViewItem owns compact icon-and-text action markup", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  let runCount = 0;
  const activeAction: IAction = {
    id: "active-terminal",
    label: "Focus Active Terminal",
    tooltip: "Focus Active Terminal",
    enabled: true,
    run: () => runCount++,
  };
  using actionBar = new ActionBar({
    ownerDocument: dom.window.document,
    actions: [activeAction],
    actionViewItemProvider: (action) => new LabelActionViewItem(action, {
      label: "cmd",
      icon: lxiconsLibrary.terminalCmd,
      ariaLabel: "Active terminal: cmd",
      tooltip: "Active terminal: cmd",
    }),
  });
  dom.window.document.body.append(actionBar.element);

  const label = actionBar.element.querySelector<HTMLButtonElement>(".zeta-action-label");
  assert.ok(label);
  assert.equal(label.classList.contains("zeta-button"), false);
  assert.equal(label.querySelector(".zeta-action-label-text")?.textContent, "cmd");
  assert.ok(label.querySelector(".zeta-action-label-icon > svg.zeta-icon"));
  assert.equal(label.getAttribute("aria-label"), "Active terminal: cmd");
  assert.equal(label.tabIndex, 0);
  label.click();
  assert.equal(runCount, 1);

  dom.window.close();
});

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

function managedHover(): IManagedHover {
  return {
    visible: false,
    show() {},
    hide() {},
    update() {},
    dispose() {},
    [Symbol.dispose]() {},
  };
}
