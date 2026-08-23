import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { createDom } from "../../browser/dom.js";

test("document-bound DOM factories create typed nested HTML", () => {
  const firstDocument = new JSDOM("<!doctype html><body></body>").window.document;
  const secondDocument = new JSDOM("<!doctype html><body></body>").window.document;
  const h = createDom(secondDocument);
  let referenced: HTMLButtonElement | undefined;

  const root = h("section", {
    className: ["root", false, "selected"],
    attributes: { role: "dialog", "aria-modal": "true" },
    properties: { tabIndex: -1, hidden: false },
    dataset: { state: "ready" },
    style: { width: "10px", backgroundColor: "red", opacity: "1" },
  }, [
    h("h2", "Title"),
    false,
    null,
    h("button", { properties: { type: "button" }, ref: value => referenced = value }, "Close"),
  ]);

  assert.equal(root.ownerDocument, secondDocument);
  assert.notEqual(root.ownerDocument, firstDocument);
  assert.equal(root.outerHTML, '<section class="root selected" role="dialog" aria-modal="true" tabindex="-1" data-state="ready" style="width: 10px; background-color: red; opacity: 1;"><h2>Title</h2><button type="button">Close</button></section>');
  assert.equal(referenced, root.querySelector("button"));
});

test("document-bound DOM factories create SVG and fragments", () => {
  const ownerDocument = new JSDOM("<!doctype html><body></body>").window.document;
  const h = createDom(ownerDocument);
  const icon = h.svg("svg", { attributes: { viewBox: "0 0 16 16" } },
    h.svg("path", { attributes: { d: "M0 0h16v16z" } }),
  );
  const fragment = h.fragment("before", icon, 3);

  assert.equal(icon.namespaceURI, "http://www.w3.org/2000/svg");
  assert.equal(fragment.ownerDocument, ownerDocument);
  assert.equal(fragment.textContent, "before3");
  assert.equal(fragment.childNodes.length, 3);
});
