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
  await expect(page.locator("#gama-text-block .zeta-structured-format-toolbar")).toHaveAttribute("data-context", "code");
  await expect(page.locator("#gama-text-block .zeta-structured-format-code-context")).toHaveText("Code block · Alpha");
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

test("Gama public distribution persists selected font, size, and emphasis formatting", async ({ page }) => {
  await page.goto("/gama.html");
  const input = page.locator("#gama-structured textarea.zeta-document-text-input").first();
  const fontFamily = page.locator("#gama-structured select[aria-label='Font family']");
  const fontSize = page.locator("#gama-structured select[aria-label='Font size']");
  await input.evaluate(element => {
    const textarea = element as HTMLTextAreaElement;
    textarea.focus();
    textarea.setSelectionRange(0, 5);
    textarea.dispatchEvent(new Event("select", { bubbles: true }));
  });
  await fontFamily.selectOption("serif");
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getStructuredFirstTextMarks())).toEqual([
    { type: "textStyle", attrs: { fontFamily: "serif" } },
  ]);
  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getStructuredSelection())).toMatchObject({ kind: "text", anchor: { offset: 0 }, head: { offset: 5 } });
  await fontSize.selectOption("18");
  await expect(fontSize).toHaveValue("18");
  const bold = page.locator("#gama-structured [data-action-id='bold'] button");
  await bold.click();

  await expect.poll(() => page.evaluate(() => window.zetaGamaIntegration.getStructuredFirstTextMarks())).toEqual([
    { type: "textStyle", attrs: { fontFamily: "serif", fontSize: 18 } },
    { type: "strong", attrs: {} },
  ]);

  const styled = page.locator("#gama-structured .zeta-document-mark-textStyle[data-font-family='serif']");
  await expect(styled).toHaveText("Title");
  await expect(styled).toHaveCSS("font-size", "18px");
  await expect(bold).toHaveAttribute("aria-pressed", "true");
});

test("Gama public distribution has the structured-editor accessibility contract", async ({ page }) => {
  await page.goto("/gama.html");
  const toolbar = page.locator("#gama-structured .zeta-structured-format-toolbar");
  const structuredInput = page.locator("#gama-structured textarea.zeta-document-text-input").first();
  await expect(toolbar).toHaveAttribute("role", "group");
  await expect(toolbar).toHaveAttribute("aria-label", "Gama document formatting");
  await expect(toolbar.locator(".zeta-structured-format-inline-actions")).toHaveAttribute("role", "toolbar");
  await expect(toolbar.locator(".zeta-structured-format-document-actions button")).toHaveCount(13);
  await expect(toolbar.locator("select[aria-label='Font family']")).toBeVisible();
  await expect(toolbar.locator("select[aria-label='Font size']")).toBeVisible();
  await expect(structuredInput).toBeVisible();

  await injectAxe(page);
  const accessibility = await getAxeResults(page, undefined, { runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"] } });
  expect(accessibility.violations.filter(violation => violation.impact === "critical")).toEqual([]);
  const contrast = await getAxeResults(page, undefined, { runOnly: { type: "rule", values: ["color-contrast"] } });
  expect(contrast.violations).toEqual([]);
});
