import assert from "node:assert/strict";
import test from "node:test";
import { codeSessionsProfile } from "../../browser/code/codeSessionsProfile.js";
import { resolveSessionsPageUrl } from "../../browser/common/sessionNavigation.js";
import { createSessionsProfile } from "../../common/sessionsProfile.js";

test("dedicated Sessions profile belongs to the Code product", () => {
  assert.equal(codeSessionsProfile.productId, "code");
  assert.equal(codeSessionsProfile.workbenchRelativePath, "../workbench/workbench.html");
});

test("Sessions navigation only resolves a sibling renderer page", () => {
  assert.equal(
    resolveSessionsPageUrl("../workbench/workbench.html", "file:///zeta/electron-browser/sessions/sessions-code.html"),
    "file:///zeta/electron-browser/workbench/workbench.html",
  );
  assert.throws(
    () => resolveSessionsPageUrl("https://example.com", "file:///zeta/electron-browser/sessions/sessions-code.html"),
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
