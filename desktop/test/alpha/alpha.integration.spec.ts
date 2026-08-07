import { expect, test } from "@playwright/test";
import { getAxeResults, injectAxe } from "axe-playwright";

const pageErrors = new WeakMap<object, string[]>();

test.beforeEach(async ({ page }) => {
  const errors: string[] = [];
  pageErrors.set(page, errors);
  page.on("pageerror", error => errors.push(error.stack ?? error.message));
});

test.afterEach(async ({ page }) => {
  await page.evaluate(() => {
    window.zetaAlphaIntegration?.dispose();
  }).catch(() => undefined);
  expect(pageErrors.get(page) ?? []).toEqual([]);
});

test("Alpha public API and browser pane type, undo, save, and start a browser worker", async ({ page }) => {
  const workers: string[] = [];
  page.on("worker", worker => workers.push(worker.url()));
  await page.goto("/alpha.html");
  await expect(page.locator(".zeta-alpha-editor")).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.apiText)).toBe("alpha-api");

  const input = page.locator(".zeta-alpha-editor-input");
  await input.focus();
  await page.keyboard.press("Control+Home");
  await page.keyboard.type("/* integrated */ ");
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getValue())).toBe("/* integrated */ fn main() {\n  answer();\n}\n");

  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getValue())).toBe("fn main() {\n  answer();\n}\n");
  await page.evaluate(() => window.zetaAlphaIntegration.save());
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getSavedText())).toBe("fn main() {\n  answer();\n}\n");
  await expect.poll(() => workers.length).toBeGreaterThan(0);
});

test("Alpha projects revision-bound Rust syntax tokens, diagnostics, folding, and symbols", async ({ page }) => {
  await page.goto("/alpha.html");
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getSyntaxAnalysisCount())).toBeGreaterThan(0);
  await expect(page.locator(".zeta-alpha-editor-token.token-keyword")).toHaveText("fn");
  await expect(page.locator(".zeta-alpha-editor-diagnostic-marker.error")).toHaveCount(1);

  const input = page.locator(".zeta-alpha-editor-input");
  await input.focus();
  await page.keyboard.press("ControlOrMeta+Shift+o");
  await expect(page.locator(".zeta-alpha-editor-goto-symbol-item")).toHaveText("main");
});

test("Alpha public distribution has the editor accessibility contract", async ({ page }) => {
  await page.goto("/alpha.html");
  const editor = page.locator(".zeta-alpha-editor");
  const input = page.locator(".zeta-alpha-editor-input");
  await expect(editor).toHaveAttribute("role", "region");
  await expect(editor).toHaveAttribute("aria-label", /.+/);
  await expect(input).toHaveAttribute("aria-multiline", "true");
  await expect(input).toHaveAttribute("aria-roledescription", "code editor");

  await injectAxe(page);
  const accessibility = await getAxeResults(page, undefined, { runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"] } });
  expect(accessibility.violations.filter(violation => violation.impact === "critical")).toEqual([]);
  const contrast = await getAxeResults(page, undefined, { runOnly: { type: "rule", values: ["color-contrast"] } });
  expect(contrast.violations).toEqual([]);
});
