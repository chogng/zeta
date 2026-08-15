import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { StatusbarPart } from "../../../../../workbench/browser/parts/statusbar/statusbarPart.js";
import { StatusbarAlignment, StatusbarService } from "../../../../../workbench/services/statusbar/browser/statusbar.js";

test("status bar entries render an icon before their text", () => {
  const document = new JSDOM("<!doctype html><body></body>").window.document;
  using service = new StatusbarService();
  using entry = service.addEntry({ icon: lxiconsLibrary.gitBranch, text: "main", ariaLabel: "Git branch main" }, { id: "test.branch", alignment: StatusbarAlignment.Left });
  using part = new StatusbarPart(service, document);
  const element = part.element.querySelector<HTMLElement>('[data-statusbar-item-id="test.branch"]');
  const label = element?.querySelector<HTMLElement>(".zeta-statusbar-item-label");

  assert.ok(element);
  assert.ok(label);
  assert.equal(label.firstElementChild?.tagName.toLowerCase(), "svg");
  assert.equal(element.textContent, "main");
  assert.equal(element.getAttribute("aria-label"), "Git branch main");
  assert.equal(label.getAttribute("role"), "button");
  assert.equal(part.minimumHeight, 35);
  assert.equal(part.maximumHeight, 35);
});

test("status bar entries support accessible icon-only presentation", () => {
  const document = new JSDOM("<!doctype html><body></body>").window.document;
  using service = new StatusbarService();
  using entry = service.addEntry({ icon: lxiconsLibrary.remote, text: "", ariaLabel: "App Server ready", tooltip: "Connected" }, { id: "test.remote", alignment: StatusbarAlignment.Left });
  using part = new StatusbarPart(service, document);
  const element = part.element.querySelector<HTMLElement>('[data-statusbar-item-id="test.remote"]');
  const label = element?.querySelector<HTMLElement>(".zeta-statusbar-item-label");

  assert.ok(element);
  assert.ok(label);
  assert.ok(element.querySelector("svg.zeta-icon"));
  assert.equal(element.textContent, "");
  assert.equal(element.getAttribute("aria-label"), "App Server ready");
  assert.equal(label.title, "Connected");
});

test("status bar entry updates retain the item shell and activate commands", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const { document } = dom.window;
  let activations = 0;
  using service = new StatusbarService();
  using entry = service.addEntry({ text: "main", run: () => activations += 1 }, { id: "test.branch", alignment: StatusbarAlignment.Left });
  using part = new StatusbarPart(service, document);
  const element = part.element.querySelector<HTMLElement>('[data-statusbar-item-id="test.branch"]');
  const label = element?.querySelector<HTMLElement>(".zeta-statusbar-item-label");

  assert.ok(element);
  assert.ok(label);
  assert.equal(label.tabIndex, 0);
  label.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true, cancelable: true }));
  assert.equal(activations, 1);

  entry.update({ text: "detached", run: () => activations += 1 });
  assert.equal(part.element.querySelector('[data-statusbar-item-id="test.branch"]'), element);
  assert.equal(element.textContent, "detached");
  dom.window.close();
});
