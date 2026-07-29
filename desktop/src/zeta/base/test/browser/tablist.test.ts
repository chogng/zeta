import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import {
  TabList,
  type TabListItem,
} from "../../browser/ui/tablist/tabList.js";
import type { IAction } from "../../common/actions.js";

test("TabList owns manual selection semantics and roving focus", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const activations: string[] = [];
  const tabList = new TabList({
    ownerDocument: dom.window.document,
    ariaLabel: "Documents",
    onActivate: (value: string) => activations.push(value),
  });
  const tabs = [
    tab("first"),
    tab("second"),
  ];
  tabList.setTabs(tabs, "first");
  dom.window.document.body.append(tabList.element);
  const elements =
    tabList.element.querySelectorAll<HTMLButtonElement>("[role='tab']");
  const tablist = tabList.element.querySelector<HTMLElement>(
    "[role='tablist']",
  );
  assert.equal(tabList.element.dataset.scrollDirection, "horizontal");
  assert.equal(
    tabList.element.querySelector(
      ".zeta-scrollbar-track-vertical",
    )?.hasAttribute("hidden"),
    true,
  );
  const first = elements[0];
  const second = elements[1];
  assert.ok(first);
  assert.ok(second);
  assert.equal(tablist?.getAttribute("role"), "tablist");
  assert.equal(tablist?.getAttribute("aria-label"), "Documents");
  assert.deepEqual(
    [...elements].map((element) => element.getAttribute("aria-selected")),
    ["true", "false"],
  );
  assert.deepEqual([...elements].map((element) => element.tabIndex), [0, -1]);
  assert.equal(first.getAttribute("aria-controls"), "first-panel");

  first.focus();
  first.dispatchEvent(keyboardEvent(dom.window, "ArrowRight"));
  assert.equal(dom.window.document.activeElement, second);
  assert.equal(second.getAttribute("aria-selected"), "false");
  assert.deepEqual(activations, []);
  second.click();
  assert.deepEqual(activations, ["second"]);

  tabList.setTabs(tabs, "second");
  assert.deepEqual(
    [...tabList.element.querySelectorAll<HTMLElement>("[role='tab']")]
      .map((element) => element.getAttribute("aria-selected")),
    ["false", "true"],
  );

  tabList.dispose();
  dom.window.close();
});

test("TabList renders IconLabel content and per-tab actions", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const closed: string[] = [];
  const tabList = new TabList({
    ownerDocument: dom.window.document,
    ariaLabel: "Editors",
    onActivate: () => undefined,
    onDelete: (value: string) => closed.push(value),
  });
  const close: IAction = {
    id: "close",
    label: "Close first",
    tooltip: "Close first",
    enabled: true,
    run: () => closed.push("first"),
  };
  tabList.setTabs([{
    ...tab("first"),
    actions: {
      ariaLabel: "First tab actions",
      items: [close],
    },
  }], "first");
  dom.window.document.body.append(tabList.element);
  const selected = tabList.element.querySelector<HTMLButtonElement>(
    "[role='tab']",
  );
  const closeButton = tabList.element.querySelector<HTMLButtonElement>(
    ".zeta-tab-actions button",
  );
  assert.ok(selected);
  assert.ok(closeButton);
  assert.equal(
    selected.querySelector(".zeta-icon-label-text")?.textContent,
    "first",
  );
  assert.equal(selected.getAttribute("aria-keyshortcuts"), "Delete");
  assert.equal(
    tabList.element.querySelector(".zeta-tab-actions")?.getAttribute("role"),
    "toolbar",
  );
  assert.equal(
    tabList.element.querySelector(".zeta-tab-actions")
      ?.getAttribute("aria-label"),
    "First tab actions",
  );
  assert.equal(closeButton.textContent, "Close first");

  selected.dispatchEvent(keyboardEvent(dom.window, "Delete"));
  closeButton.click();
  assert.deepEqual(closed, ["first", "first"]);

  tabList.dispose();
  dom.window.close();
});

test("TabList rejects ambiguous item and selection identities", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tabList = new TabList({
    ownerDocument: dom.window.document,
    ariaLabel: "Tabs",
    onActivate: () => undefined,
  });
  assert.throws(
    () => tabList.setTabs([tab("same"), tab("same")], "same"),
    /Duplicate TabList item ID/,
  );
  assert.throws(
    () => tabList.setTabs([tab("first")], "missing"),
    /Selected TabList item is not available/,
  );

  tabList.dispose();
  dom.window.close();
});

function tab(id: string): TabListItem<string> {
  return {
    id,
    value: id,
    label: id,
    tabId: `${id}-tab`,
    panelId: `${id}-panel`,
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
