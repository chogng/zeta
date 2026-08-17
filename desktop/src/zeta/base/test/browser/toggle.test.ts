import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Checkbox, Switch, Toggle } from "../../browser/ui/toggle/toggle.js";

test("Toggle and Checkbox expose shared native boolean state", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const changes: boolean[] = [];
  using checkbox = new Checkbox({
    ownerDocument: dom.window.document,
    label: "Underline links",
    onChange: checked => changes.push(checked),
  });
  dom.window.document.body.append(checkbox.element);

  assert.equal(checkbox.element.classList.contains("zeta-checkbox"), true);
  assert.equal(checkbox.input.type, "checkbox");
  assert.equal(checkbox.checked, false);

  checkbox.input.click();

  assert.equal(checkbox.checked, true);
  assert.equal(checkbox.element.classList.contains("checked"), true);
  assert.deepEqual(changes, [true]);

  checkbox.checked = false;
  assert.equal(checkbox.element.classList.contains("checked"), false);
  dom.window.close();
});

test("Switch projects the shared state as a switch control", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  using toggle = new Toggle({
    ownerDocument: dom.window.document,
    ariaLabel: "Generic toggle",
  });
  using switchControl = new Switch({
    ownerDocument: dom.window.document,
    ariaLabel: "Reduce motion",
    checked: true,
  });
  dom.window.document.body.append(toggle.element, switchControl.element);

  assert.equal(toggle.input.getAttribute("role"), null);
  assert.equal(switchControl.input.getAttribute("role"), "switch");
  assert.equal(switchControl.input.getAttribute("aria-checked"), "true");
  assert.equal(switchControl.element.querySelector(".zeta-switch-track"), switchControl.track);

  switchControl.input.click();

  assert.equal(switchControl.checked, false);
  assert.equal(switchControl.input.getAttribute("aria-checked"), "false");
  assert.equal(switchControl.element.classList.contains("checked"), false);
  dom.window.close();
});

test("Toggle can place its content before the control", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const content = dom.window.document.createElement("span");
  content.textContent = "Minimap";
  using switchControl = new Switch({
    ownerDocument: dom.window.document,
    ariaLabel: "Minimap",
    content,
    contentPlacement: "before-control",
  });
  dom.window.document.body.append(switchControl.element);

  assert.equal(switchControl.element.classList.contains("zeta-toggle-content-before-control"), true);
  assert.equal(switchControl.element.querySelector(".zeta-toggle-content")?.firstElementChild, content);
  assert.equal(switchControl.element.children[1]?.classList.contains("zeta-toggle-content"), true);
  assert.equal(switchControl.element.children[2], switchControl.track);
  dom.window.close();
});
