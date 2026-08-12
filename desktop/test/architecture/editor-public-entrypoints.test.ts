import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const editorRoot = resolve(findDesktopRoot(import.meta.dirname), "src/zeta/editor");

test("flat editor exposes VS Code-shaped public entrypoints and product bundles", () => {
  for (const entrypoint of ["editor.api.ts", "editor.code.all.ts", "editor.academic.all.ts", "editor.all.ts", "editor.main.ts", "editor.worker.start.ts"]) {
    assert.equal(exists(join(editorRoot, entrypoint)), true, entrypoint);
  }
  assert.equal(exists(join(editorRoot, "alpha")), false, "alpha directory");
  assert.equal(exists(join(editorRoot, "gama")), false, "gama directory");
});

test("public editor entrypoints retain distinct API, contribution, main, and worker roles", () => {
  const api = readFileSync(join(editorRoot, "editor.api.ts"), "utf8");
  const codeBundle = readFileSync(join(editorRoot, "editor.code.all.ts"), "utf8");
  const academicBundle = readFileSync(join(editorRoot, "editor.academic.all.ts"), "utf8");
  const all = readFileSync(join(editorRoot, "editor.all.ts"), "utf8");
  const main = readFileSync(join(editorRoot, "editor.main.ts"), "utf8");
  const worker = readFileSync(join(editorRoot, "editor.worker.start.ts"), "utf8");
  const analysisWorker = readFileSync(join(editorRoot, "browser/language/syntaxWorkerMain.ts"), "utf8");
  const completionWorker = readFileSync(join(editorRoot, "browser/language/languageCompletionWorkerMain.ts"), "utf8");
  assert.match(api, /TextModel/u);
  assert.match(api, /DocumentModel/u);
  assert.doesNotMatch(api, /workbench|browser|contrib/u);
  assert.match(codeBundle, /contrib\/editor\.contribution/u);
  assert.doesNotMatch(codeBundle, /contrib\/academic/u);
  assert.match(academicBundle, /contrib\/academic\/browser\/academicEditor\.contribution/u);
  assert.doesNotMatch(academicBundle, /contrib\/editor\.contribution/u);
  assert.match(all, /editor\.code\.all/u);
  assert.match(all, /editor\.academic\.all/u);
  assert.match(main, /import "\.\/editor\.all\.js"/u);
  assert.match(main, /export \* from "\.\/editor\.api\.js"/u);
  assert.match(worker, /export function start/u);
  assert.match(analysisWorker, /languageWorker\.start/u);
  assert.match(completionWorker, /languageWorker\.start/u);
});

function exists(file: string): boolean {
  try {
    return statSync(file).isFile();
  } catch {
    return false;
  }
}
