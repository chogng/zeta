import assert from "node:assert/strict";
import test from "node:test";
import { defaultWorkbenchProfile } from "../../../workbench/browser/defaultWorkbenchProfile.js";

test("build modes share one default Workbench profile", () => {
  assert.equal(defaultWorkbenchProfile.id, "default");
  assert.equal(defaultWorkbenchProfile.label, "Workbench");
  assert.equal("productId" in defaultWorkbenchProfile, false);
  assert.equal(defaultWorkbenchProfile.layout.auxiliarybar.visible, true);
  assert.equal(defaultWorkbenchProfile.composition.panel, "zeta.panel.terminal");
});

test("the shared Workbench profile is immutable at its boundary", () => {
  assert.throws(() => {
    (defaultWorkbenchProfile as { id: string }).id = "academic";
  }, TypeError);
  assert.throws(() => {
    (defaultWorkbenchProfile.layout.sidebar as { width: number }).width = 999;
  }, TypeError);
  assert.throws(() => {
    (defaultWorkbenchProfile.composition as { panel: string }).panel = "zeta.panel.problems";
  }, TypeError);
});
