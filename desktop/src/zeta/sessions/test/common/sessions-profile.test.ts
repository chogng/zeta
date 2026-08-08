import assert from "node:assert/strict";
import test from "node:test";
import { academicSessionsProfile } from "../../browser/academic/academicSessionsProfile.js";
import { codeSessionsProfile } from "../../browser/code/codeSessionsProfile.js";
import { resolveSessionsPageUrl } from "../../browser/common/sessionNavigation.js";
import { createSessionsProfile } from "../../common/sessionsProfile.js";

test("dedicated Sessions profiles own the two product identities", () => {
  assert.equal(codeSessionsProfile.productId, "code");
  assert.equal(codeSessionsProfile.workbenchRelativePath, "../workbench/workbench-code.html");
  assert.equal(academicSessionsProfile.productId, "academic");
  assert.equal(academicSessionsProfile.workbenchRelativePath, "../workbench/workbench-academic.html");
  assert.notEqual(codeSessionsProfile.titlebarActionId, academicSessionsProfile.titlebarActionId);
});

test("Sessions navigation only resolves a sibling renderer page", () => {
  assert.equal(
    resolveSessionsPageUrl("../workbench/workbench-academic.html", "file:///zeta/electron-browser/sessions/sessions-academic.html"),
    "file:///zeta/electron-browser/workbench/workbench-academic.html",
  );
  assert.throws(
    () => resolveSessionsPageUrl("https://example.com", "file:///zeta/electron-browser/sessions/sessions-academic.html"),
    /sibling renderer directory/,
  );
});

test("Sessions profiles reject a non-sibling Workbench return path", () => {
  assert.throws(
    () => createSessionsProfile({
      id: "invalid",
      productId: "code",
      label: "Invalid",
      titlebarActionId: "zeta.invalid",
      workbenchRelativePath: "../../outside.html",
    }),
    /sibling Workbench page/,
  );
});
