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
    window.zetaGamaIntegration?.dispose();
  }).catch(() => undefined);
  expect(pageErrors.get(page) ?? []).toEqual([]);
});

test("Gama public API, structured editing, and Alpha-backed textBlock bridge run in real browsers", async ({ page }) => {
  await page.goto("/gama.html");
  await expect(page.locator("#gama-text-block .zeta-alpha-editor")).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.apiDocumentType)).toBe("doc");

  const textBlockInput = page.locator("#gama-text-block .zeta-alpha-editor-input");
  await textBlockInput.focus();
  await page.keyboard.press("Control+Home");
  await page.keyboard.type("// bridge\n");
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getTextBlockText())).toBe("// bridge\nconst gama = 1;");
  await page.evaluate(() => window.zetaGamaIntegration.saveTextBlock());
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getSavedTextBlock())).toContain("// bridge\\nconst gama = 1;");

  const structuredInput = page.locator("#gama-structured textarea.zeta-document-text-input").first();
  await structuredInput.focus();
  await page.keyboard.press("End");
  await page.keyboard.press("Enter");
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getStructuredBlockTexts())).toEqual(["Title", "", "Body"]);
  await page.keyboard.press("Control+z");
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getStructuredBlockTexts())).toEqual(["Title", "Body"]);
});

test("Gama public distribution has the structured-editor accessibility contract", async ({ page }) => {
  await page.goto("/gama.html");
  const toolbar = page.locator("#gama-structured .zeta-document-block-toolbar");
  const structuredInput = page.locator("#gama-structured textarea.zeta-document-text-input").first();
  await expect(toolbar).toHaveAttribute("role", "toolbar");
  await expect(toolbar).toHaveAttribute("aria-label", "Block formatting");
  await expect(toolbar.locator("button")).toHaveCount(13);
  await expect(structuredInput).toBeVisible();

  await injectAxe(page);
  const accessibility = await getAxeResults(page, undefined, { runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"] } });
  expect(accessibility.violations.filter(violation => violation.impact === "critical")).toEqual([]);
  const contrast = await getAxeResults(page, undefined, { runOnly: { type: "rule", values: ["color-contrast"] } });
  expect(contrast.violations).toEqual([]);
});
