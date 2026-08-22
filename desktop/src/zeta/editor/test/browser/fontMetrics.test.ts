import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DomTextMeasurer } from "../../browser/measurement/fontMetrics.js";

test("DomTextMeasurer resolves computed spacing and tab stops", () => {
  const dom = new JSDOM("<!doctype html><body><div></div></body>");
  const context = {
    font: "",
    textBaseline: "",
    measureText: (text: string) => ({ width: [...text].length * 5 }),
  };
  Object.defineProperty(
    dom.window.HTMLCanvasElement.prototype,
    "getContext",
    {
      configurable: true,
      value: () => context,
    },
  );
  const reference = dom.window.document.querySelector("div");
  assert.ok(reference);
  reference.style.fontFamily = "Aster Mono";
  reference.style.fontSize = "10px";
  reference.style.fontWeight = "400";
  reference.style.letterSpacing = "1px";
  reference.style.paddingLeft = "2px";
  reference.style.paddingRight = "3px";
  reference.style.tabSize = "4";
  const measurer = new DomTextMeasurer(reference);

  assert.equal(measurer.horizontalPadding, 5);
  assert.equal(measurer.contentLeftPadding, 2);
  assert.equal(measurer.measureLineWidth("ab"), 12);
  assert.equal(measurer.measureLineWidth("a\tb"), 26);
  assert.equal(measurer.refresh(), false);

  reference.style.fontSize = "12px";
  assert.equal(measurer.refresh(), true);
  assert.match(context.font, /12px/);

  dom.window.close();
});
