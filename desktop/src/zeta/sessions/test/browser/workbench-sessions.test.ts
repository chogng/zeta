import assert from "node:assert/strict";
import test from "node:test";
import { academicWorkbenchSession } from "../../browser/academicWorkbenchSession.js";
import { codeWorkbenchSession } from "../../browser/codeWorkbenchSession.js";
import { completeWorkbenchSession } from "../../browser/completeWorkbenchSession.js";

test("product sessions provide distinct Workbench layout profiles", () => {
  assert.equal(codeWorkbenchSession.id, "code");
  assert.equal(academicWorkbenchSession.id, "academic");
  assert.equal(completeWorkbenchSession.id, "complete");
  assert.equal(codeWorkbenchSession.layout.auxiliarybar.visible, true);
  assert.equal(academicWorkbenchSession.layout.auxiliarybar.visible, false);
  assert.notEqual(codeWorkbenchSession.layout.sidebar.width, academicWorkbenchSession.layout.sidebar.width);
  assert.equal(codeWorkbenchSession.composition.panel, "zeta.panel.terminal");
  assert.equal(academicWorkbenchSession.composition.panel, "zeta.panel.problems");
});

test("session layout profiles are immutable at the profile boundary", () => {
  assert.throws(() => {
    (codeWorkbenchSession as { id: string }).id = "academic";
  }, TypeError);
  assert.throws(() => {
    (codeWorkbenchSession.layout.sidebar as { width: number }).width = 999;
  }, TypeError);
  assert.throws(() => {
    (codeWorkbenchSession.composition as { panel: string }).panel = "zeta.panel.problems";
  }, TypeError);
});
