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

  assert.ok(element);
  assert.equal(element.firstElementChild?.tagName.toLowerCase(), "svg");
  assert.equal(element.textContent, "main");
  assert.equal(element.getAttribute("aria-label"), "Git branch main");
});

test("status bar entries support accessible icon-only presentation", () => {
  const document = new JSDOM("<!doctype html><body></body>").window.document;
  using service = new StatusbarService();
  using entry = service.addEntry({ icon: lxiconsLibrary.remote, text: "", ariaLabel: "App Server ready", tooltip: "Connected" }, { id: "test.remote", alignment: StatusbarAlignment.Left });
  using part = new StatusbarPart(service, document);
  const element = part.element.querySelector<HTMLElement>('[data-statusbar-item-id="test.remote"]');

  assert.ok(element);
  assert.ok(element.querySelector("svg.zeta-icon"));
  assert.equal(element.textContent, "");
  assert.equal(element.getAttribute("aria-label"), "App Server ready");
  assert.equal(element.title, "Connected");
});
