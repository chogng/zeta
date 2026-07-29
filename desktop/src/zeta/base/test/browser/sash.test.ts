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
  PointerEvent: browserEnvironment.window.MouseEvent,
})) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    value,
  });
}

const { Sash, SashSettingsBinding } = await import("../../browser/ui/sash/sash.js");

test("Sash settings are scoped to a subtree and clean up their projection", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const root = dom.window.document.querySelector("main");
  assert.ok(root);
  const sash = dom.window.document.createElement("div");
  sash.className = "zeta-sash";
  root.append(sash);

  {
    using binding = new SashSettingsBinding(root);
    binding.update({
      dragAreaSize: 12,
      hoverFeedbackSize: 6,
      hoverDelay: 150,
    });
    assert.equal(
      dom.window.getComputedStyle(sash)
        .getPropertyValue("--zeta-sash-drag-area-size"),
      "12px",
    );
    assert.equal(
      dom.window.getComputedStyle(sash)
        .getPropertyValue("--zeta-sash-hover-feedback-size"),
      "6px",
    );
    assert.equal(
      dom.window.getComputedStyle(sash)
        .getPropertyValue("--zeta-sash-hover-delay"),
      "150ms",
    );
  }

  assert.equal(root.className, "");
  assert.equal(dom.window.document.head.querySelector("style"), null);
  dom.window.close();
});

test("Sash settings reject invalid dimensions and delays", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const root = dom.window.document.querySelector("main");
  assert.ok(root);
  using binding = new SashSettingsBinding(root);

  assert.throws(
    () => binding.update({
      dragAreaSize: 0,
      hoverFeedbackSize: 1,
      hoverDelay: 0,
    }),
    /drag area size/,
  );
  assert.throws(
    () => binding.update({
      dragAreaSize: 4,
      hoverFeedbackSize: 1,
      hoverDelay: -1,
    }),
    /hover delay/,
  );
  dom.window.close();
});

test("Sash applies configured hover feedback and active drag state", () => {
  const dom = new JSDOM("<!doctype html><body><main></main></body>");
  const root = dom.window.document.querySelector("main");
  assert.ok(root);
  using binding = new SashSettingsBinding(root);
  binding.update({
    dragAreaSize: 4,
    hoverFeedbackSize: 4,
    hoverDelay: 0,
  });
  using sash = new Sash("vertical", dom.window.document);
  root.append(sash.element);

  sash.element.dispatchEvent(new dom.window.MouseEvent("pointerenter"));
  assert.equal(sash.element.classList.contains("zeta-sash-hover"), true);
  sash.element.dispatchEvent(new dom.window.MouseEvent("pointerleave"));
  assert.equal(sash.element.classList.contains("zeta-sash-hover"), false);

  sash.element.dispatchEvent(new dom.window.MouseEvent("pointerdown", {
    bubbles: true,
    button: 0,
  }));
  assert.equal(sash.element.classList.contains("zeta-sash-active"), true);
  dom.window.dispatchEvent(new dom.window.MouseEvent("pointerup", {
    bubbles: true,
    button: 0,
  }));
  assert.equal(sash.element.classList.contains("zeta-sash-active"), false);
  dom.window.close();
});
