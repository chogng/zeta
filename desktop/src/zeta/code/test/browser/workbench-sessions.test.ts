import assert from "node:assert/strict";
import test from "node:test";
import { defaultWorkbenchSession } from "../../../workbench/browser/defaultWorkbenchSession.js";

test("build modes share one default Workbench profile", () => {
  assert.equal(defaultWorkbenchSession.id, "default");
  assert.equal(defaultWorkbenchSession.label, "Workbench");
  assert.equal("productId" in defaultWorkbenchSession, false);
  assert.equal(defaultWorkbenchSession.layout.auxiliarybar.visible, true);
  assert.equal(defaultWorkbenchSession.composition.panel, "zeta.panel.terminal");
});

test("the shared Workbench layout profile is immutable at its boundary", () => {
  assert.throws(() => {
    (defaultWorkbenchSession as { id: string }).id = "academic";
  }, TypeError);
  assert.throws(() => {
    (defaultWorkbenchSession.layout.sidebar as { width: number }).width = 999;
  }, TypeError);
  assert.throws(() => {
    (defaultWorkbenchSession.composition as { panel: string }).panel = "zeta.panel.problems";
  }, TypeError);
});
