import assert from "node:assert/strict";
import test from "node:test";
import { registerEditorContribution } from "../../browser/editorContribution.js";
import { getEditorContributions } from "../../browser/editorContribution.js";

test("editor contributions retain bundle registration order and stable identity", async () => {
  const before = getEditorContributions().map(contribution => contribution.id);
  assert.equal(before.includes("editor.contrib.find"), false);

  await import("../../contrib/find/browser/find.contribution.js");
  const after = getEditorContributions().map(contribution => contribution.id);
  assert.deepEqual(after, [...before, "editor.contrib.find"]);
  const contribution = getEditorContributions().find(candidate => candidate.id === "editor.contrib.find");
  assert.ok(contribution);
  assert.doesNotThrow(() => contribution.install({ kind: "document" } as never));

  assert.throws(() => registerEditorContribution({ id: "editor.contrib.find", install() {} }), /Duplicate editor contribution/);
  assert.deepEqual(getEditorContributions().map(contribution => contribution.id), after);
});
