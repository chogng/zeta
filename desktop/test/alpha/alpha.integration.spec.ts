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
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getValue())).toBe("/* integrated */ const answer = 42;\nconsole.log(answer);");

  await page.keyboard.press("ControlOrMeta+z");
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getValue())).toBe("const answer = 42;\nconsole.log(answer);");
  await page.evaluate(() => window.zetaAlphaIntegration.save());
  await expect.poll(() => page.evaluate(() => window.zetaAlphaIntegration.getSavedText())).toBe("const answer = 42;\nconsole.log(answer);");
  await expect.poll(() => workers.length).toBeGreaterThan(0);
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
