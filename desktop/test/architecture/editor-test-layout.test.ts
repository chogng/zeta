import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";
import { findDesktopRoot } from "./testPaths.js";

const desktopRoot = findDesktopRoot(import.meta.dirname);
const editorRoot = join(desktopRoot, "src/zeta/editor");
const browserIntegrationRoot = join(desktopRoot, "test/editor/browser");
const desktopPackage = JSON.parse(readFileSync(join(desktopRoot, "package.json"), "utf8")) as { scripts?: Record<string, string> };

test("Aster unit tests follow the flat editor common, browser, and contrib layout", () => {
  assert.equal(exists(join(desktopRoot, "test/monaco")), false);
  assert.equal(exists(join(desktopRoot, "test/editor/run-unit.mjs")), true);
  assert.equal(exists(join(editorRoot, "test/common/textModel.test.ts")), true);
  assert.equal(exists(join(desktopRoot, "src/zeta/workbench/contrib/codeEditor/test/browser/codeEditorPane.test.ts")), true);
  assert.equal(exists(join(editorRoot, "contrib/find/test/browser/findController.test.ts")), true);
  assert.equal(exists(join(editorRoot, "test/common/document-model.test.ts")), true);
  assert.equal(exists(join(desktopRoot, "src/zeta/workbench/contrib/documentEditor/test/browser/documentEditorPane.test.ts")), true);
});

test("Aster browser integration is flat and named after concrete model mount points", () => {
  for (const file of ["textModel.html", "textModel.integration.ts", "textModel.integration.spec.ts", "documentModel.html", "documentModel.integration.ts", "documentModel.integration.spec.ts", "memoryTextFiles.ts", "playwright.config.ts", "vite.config.ts"]) {
    assert.equal(exists(join(browserIntegrationRoot, file)), true, file);
  }
  assert.equal(exists(join(desktopRoot, "test/alpha")), false);
  assert.equal(exists(join(desktopRoot, "test/gama")), false);
  const textModelIntegration = readFileSync(join(browserIntegrationRoot, "textModel.integration.spec.ts"), "utf8");
  const documentModelIntegration = readFileSync(join(browserIntegrationRoot, "documentModel.integration.spec.ts"), "utf8");
  const config = readFileSync(join(browserIntegrationRoot, "playwright.config.ts"), "utf8");
  assert.match(textModelIntegration, /zetaTextModelIntegration/u);
  assert.match(documentModelIntegration, /zetaDocumentModelIntegration/u);
  assert.match(textModelIntegration, /axe-playwright/u);
  assert.match(documentModelIntegration, /axe-playwright/u);
  assert.match(config, /name:\s*"chromium"/u);
  assert.doesNotMatch(config, /firefox/u);
});

test("browser integrations import the stable API and only their mode bundle", () => {
  const textModelIntegration = readFileSync(join(browserIntegrationRoot, "textModel.integration.ts"), "utf8");
  const documentModelIntegration = readFileSync(join(browserIntegrationRoot, "documentModel.integration.ts"), "utf8");
  assert.match(textModelIntegration, /editor\/editor\.api\.js/u);
  assert.match(textModelIntegration, /editor\/editor\.code\.all\.js/u);
  assert.doesNotMatch(textModelIntegration, /editor\.(?:main|academic\.all)\.js/u);
  assert.match(documentModelIntegration, /editor\/editor\.api\.js/u);
  assert.match(documentModelIntegration, /editor\/editor\.academic\.all\.js/u);
  assert.doesNotMatch(documentModelIntegration, /editor\.(?:main|code\.all)\.js/u);
});

test("desktop exposes one editor browser test entrypoint", () => {
  assert.equal(desktopPackage.scripts?.["test:editor:browser"], "tsc -p test/editor/browser/tsconfig.json && node scripts/run-editor-browser-tests.mjs");
  assert.equal(desktopPackage.scripts?.["test:alpha"], undefined);
  assert.equal(desktopPackage.scripts?.["test:gama"], undefined);
});

function exists(file: string): boolean {
  try {
    statSync(file);
    return true;
  } catch {
    return false;
  }
}
