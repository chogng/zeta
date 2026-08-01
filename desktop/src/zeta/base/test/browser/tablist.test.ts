import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { TAB_CLOSE_ACTION_ID, TabList, type TabListItem } from "../../browser/ui/tablist/tabList.js";
import type { IAction } from "../../common/actions.js";
import { register } from "../../common/icon.js";

const customCloseIcon = register("tablist-test-close", () => '<svg viewBox="0 0 16 16" data-test-icon="custom-close"><path d="M2 2h12v12H2z"/></svg>');

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
  assert.deepEqual(
    [...tabList.element.querySelectorAll(".zeta-tab")].map((element) => element.classList.contains("checked")),
    [true, false],
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
  assert.deepEqual(
    [...tabList.element.querySelectorAll(".zeta-tab")].map((element) => element.classList.contains("checked")),
    [false, true],
  );

  tabList.dispose();
  dom.window.close();
});

test("TabList renders IconLabel content, custom actions, and its standard close action", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const closed: string[] = [];
  const tabList = new TabList({
    ownerDocument: dom.window.document,
    ariaLabel: "Editors",
    onActivate: () => undefined,
    onClose: (value: string) => closed.push(value),
    closeActionIcon: customCloseIcon,
  });
  const pin: IAction = {
    id: "pin",
    label: "Pin first",
    tooltip: "Pin first",
    enabled: true,
    run: () => undefined,
  };
  tabList.setTabs([{
    ...tab("first"),
    actions: {
      ariaLabel: "First tab actions",
      items: [pin],
    },
  }], "first");
  dom.window.document.body.append(tabList.element);
  const selected = tabList.element.querySelector<HTMLButtonElement>(
    "[role='tab']",
  );
  const closeButton = tabList.element.querySelector<HTMLButtonElement>(
    ".zeta-tab-close-action button",
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
  assert.deepEqual(
    [...tabList.element.querySelectorAll<HTMLElement>(".zeta-tab-actions [data-action-id]")]
      .map((item) => item.dataset.actionId),
    ["pin", TAB_CLOSE_ACTION_ID],
  );
  assert.equal(closeButton.title, "Close first");
  assert.equal(closeButton.closest(".zeta-action-view-item")?.getAttribute("data-action-id"), TAB_CLOSE_ACTION_ID);
  assert.equal(closeButton.querySelector("svg.zeta-icon")?.getAttribute("data-test-icon"), "custom-close");

  selected.dispatchEvent(keyboardEvent(dom.window, "Delete"));
  closeButton.click();
  assert.deepEqual(closed, ["first", "first"]);

  tabList.dispose();
  dom.window.close();
});

test("TabList supports vertical ActionBar navigation and scrolling", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const tabList = new TabList({
    ownerDocument: dom.window.document,
    ariaLabel: "Terminal instances",
    orientation: "vertical",
    onActivate: () => undefined,
  });
  tabList.setTabs([{ ...tab("first"), state: "running" }], "first");
  dom.window.document.body.append(tabList.element);
  const actionBar = tabList.element.querySelector<HTMLElement>("[role='tablist']");
  assert.equal(tabList.element.dataset.scrollDirection, "vertical");
  assert.equal(actionBar?.getAttribute("aria-orientation"), "vertical");
  assert.equal(actionBar?.classList.contains("vertical"), true);
  assert.equal(actionBar?.querySelector(".zeta-tab.checked") !== null, true);
  assert.equal(actionBar?.querySelector<HTMLElement>(".zeta-tab")?.dataset.state, "running");

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
