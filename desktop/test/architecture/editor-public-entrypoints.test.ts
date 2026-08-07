import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";

const editorRoot = resolve(import.meta.dirname, "../../../..", "src/zeta/editor");

test("Alpha and Gama expose VS Code-shaped public editor entrypoints", () => {
  const alpha = join(editorRoot, "alpha");
  const gama = join(editorRoot, "gama");
  for (const entrypoint of ["editor.api.ts", "editor.all.ts", "editor.main.ts", "editor.worker.start.ts"]) {
    assert.equal(exists(join(alpha, entrypoint)), true, `alpha/${entrypoint}`);
  }
  for (const entrypoint of ["editor.api.ts", "editor.all.ts", "editor.main.ts"]) {
    assert.equal(exists(join(gama, entrypoint)), true, `gama/${entrypoint}`);
  }
  assert.equal(exists(join(gama, "editor.worker.start.ts")), false);
});

test("public editor entrypoints retain distinct API, contribution, main, and worker roles", () => {
  const alpha = join(editorRoot, "alpha");
  const gama = join(editorRoot, "gama");
  const alphaApi = readFileSync(join(alpha, "editor.api.ts"), "utf8");
  const alphaMain = readFileSync(join(alpha, "editor.main.ts"), "utf8");
  const alphaWorker = readFileSync(join(alpha, "editor.worker.start.ts"), "utf8");
  const gamaApi = readFileSync(join(gama, "editor.api.ts"), "utf8");
  const gamaMain = readFileSync(join(gama, "editor.main.ts"), "utf8");
  const analysisWorker = readFileSync(join(alpha, "browser/language/syntaxWorkerMain.ts"), "utf8");
  const completionWorker = readFileSync(join(alpha, "browser/language/languageCompletionWorkerMain.ts"), "utf8");
  assert.match(alphaApi, /TextModel/u);
  assert.doesNotMatch(alphaApi, /workbench|browser|contrib/u);
  assert.match(gamaApi, /DocumentModel/u);
  assert.doesNotMatch(gamaApi, /workbench|browser|contrib/u);
  for (const main of [alphaMain, gamaMain]) {
    assert.match(main, /import "\.\/editor\.all\.js"/u);
    assert.match(main, /export \* from "\.\/editor\.api\.js"/u);
  }
  assert.match(alphaWorker, /export function start/u);
  assert.match(analysisWorker, /editor\.worker\.start/u);
  assert.match(completionWorker, /editor\.worker\.start/u);
});

function exists(file: string): boolean {
  try {
    return statSync(file).isFile();
  } catch {
    return false;
  }
}
