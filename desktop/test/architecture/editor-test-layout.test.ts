import assert from "node:assert/strict";
import { readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import test from "node:test";

const desktopRoot = resolve(import.meta.dirname, "../../../..");
const editorRoot = join(desktopRoot, "src/zeta/editor");
const alphaIntegrationRoot = join(desktopRoot, "test/alpha");
const gamaIntegrationRoot = join(desktopRoot, "test/gama");
const rootPackage = JSON.parse(readFileSync(join(desktopRoot, "../package.json"), "utf8")) as { scripts?: Record<string, string> };

test("editor unit tests remain co-located with Alpha and Gama source domains", () => {
  assert.equal(exists(join(desktopRoot, "test/monaco")), false);
  assert.equal(exists(join(desktopRoot, "test/editor")), false);
  assert.equal(exists(join(editorRoot, "alpha/test/common/textModel.test.ts")), true);
  assert.equal(exists(join(editorRoot, "alpha/test/browser/editorPane.test.ts")), true);
  assert.equal(exists(join(editorRoot, "alpha/contrib/find/test/browser/findController.test.ts")), true);
  assert.equal(exists(join(editorRoot, "gama/test/common/document-model.test.ts")), true);
  assert.equal(exists(join(editorRoot, "gama/test/browser/editorWidget.test.ts")), true);
});

test("Alpha browser integration owns only Alpha runtime coverage", () => {
  for (const file of ["alpha.html", "alpha.integration.ts", "alpha.integration.spec.ts", "memoryTextFiles.ts", "playwright.config.ts", "vite.config.ts"]) {
    assert.equal(exists(join(alphaIntegrationRoot, file)), true, file);
  }
  const integration = readFileSync(join(alphaIntegrationRoot, "alpha.integration.spec.ts"), "utf8");
  const config = readFileSync(join(alphaIntegrationRoot, "playwright.config.ts"), "utf8");
  assert.match(integration, /Alpha public API/u);
  assert.match(integration, /zetaAlphaIntegration/u);
  assert.match(integration, /axe-playwright/u);
  assert.match(config, /name:\s*"chromium"/u);
  assert.match(config, /name:\s*"firefox"/u);
  assert.doesNotMatch(integration, /Gama|gama/u);
});

test("Gama browser integration owns only Gama runtime coverage", () => {
  for (const file of ["gama.html", "gama.integration.ts", "gama.integration.spec.ts", "memoryTextFiles.ts", "playwright.config.ts", "vite.config.ts"]) {
    assert.equal(exists(join(gamaIntegrationRoot, file)), true, file);
  }
  const integration = readFileSync(join(gamaIntegrationRoot, "gama.integration.spec.ts"), "utf8");
  const config = readFileSync(join(gamaIntegrationRoot, "playwright.config.ts"), "utf8");
  assert.match(integration, /Gama public API/u);
  assert.match(integration, /zetaGamaIntegration/u);
  assert.match(integration, /axe-playwright/u);
  assert.match(config, /name:\s*"chromium"/u);
  assert.match(config, /name:\s*"firefox"/u);
  assert.doesNotMatch(integration, /zetaAlphaIntegration/u);
});

test("root test entrypoints keep Alpha and Gama browser suites independently executable", () => {
  assert.equal(rootPackage.scripts?.["test:desktop:alpha"], "corepack pnpm --dir desktop test:alpha");
  assert.equal(rootPackage.scripts?.["test:desktop:gama"], "corepack pnpm --dir desktop test:gama");
});

function exists(file: string): boolean {
  try {
    statSync(file);
    return true;
  } catch {
    return false;
  }
}
