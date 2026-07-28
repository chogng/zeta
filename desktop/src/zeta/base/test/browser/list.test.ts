import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { List } from "../../browser/ui/list/list.js";

test("List renders in its owner document and owns active navigation", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const list = new List<string>({
    ownerDocument: dom.window.document,
    ariaLabel: "Choices",
    renderItem: (item) => {
      const label = dom.window.document.createElement("span");
      label.textContent = item;
      return label;
    },
  });
  const active: (string | undefined)[] = [];
  list.onDidChangeActive(({ item }) => active.push(item));

  list.items = ["First", "Second", "Third"];
  assert.equal(list.element.ownerDocument, dom.window.document);
  assert.equal(list.element.getAttribute("role"), "listbox");
  assert.equal(list.element.getAttribute("aria-label"), "Choices");
  assert.equal(list.activeItem, "First");
  assert.equal(
    list.element.querySelector(".is-active")?.textContent,
    "First",
  );

  list.focusPrevious();
  assert.equal(list.activeItem, "Third");
  list.focusNext();
  assert.equal(list.activeItem, "First");
  assert.deepEqual(active, ["First", "Third", "First"]);

  list.dispose();
  dom.window.close();
});

test("List maps mouse interaction to activation and acceptance", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const list = new List<string>({
    ownerDocument: dom.window.document,
    renderItem: (item) => {
      const label = dom.window.document.createElement("span");
      label.textContent = item;
      return label;
    },
  });
  list.items = ["First", "Second"];
  const accepted: string[] = [];
  list.onDidAccept(({ item }) => accepted.push(item));
  const second = list.element.querySelectorAll<HTMLElement>(
    ".zeta-list-row",
  )[1];
  assert.ok(second);

  second.dispatchEvent(new dom.window.MouseEvent("mousemove", {
    bubbles: true,
  }));
  assert.equal(list.activeItem, "Second");
  second.dispatchEvent(new dom.window.MouseEvent("click", {
    bubbles: true,
  }));
  assert.deepEqual(accepted, ["Second"]);
  assert.equal(second.getAttribute("aria-selected"), "true");

  list.items = [];
  assert.equal(list.activeItem, undefined);
  assert.equal(list.element.childElementCount, 0);

  list.dispose();
  dom.window.close();
});
