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
		window.zetaTextModelIntegration?.dispose();
	}).catch(() => undefined);
	expect(pageErrors.get(page) ?? []).toEqual([]);
});

test("text-model editor public API, pane, undo, save, and browser worker", async ({ page }) => {
	const workers: string[] = [];
	page.on("worker", worker => workers.push(worker.url()));
	await page.goto("/textModel.html");
	await expect(page.locator(".aster-editor")).toBeVisible();
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.apiText)).toBe("editor-api");

	const input = page.locator(".aster-editor-input");
	await input.focus();
	await page.keyboard.press("Control+Home");
	await page.keyboard.type("/* integrated */ ");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getValue())).toBe("/* integrated */ fn main() {\n  answer();\n}\n");

	await page.keyboard.press("ControlOrMeta+z");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getValue())).toBe("fn main() {\n  answer();\n}\n");
	await page.evaluate(() => window.zetaTextModelIntegration.save());
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getSavedText())).toBe("fn main() {\n  answer();\n}\n");
	await expect.poll(() => workers.length).toBeGreaterThan(0);
});

test("text-model editor projects revision-bound Rust syntax, diagnostics, folding, and symbols", async ({ page }) => {
	await page.goto("/textModel.html");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getSyntaxAnalysisCount())).toBeGreaterThan(0);
	await expect(page.locator(".aster-editor-token.token-keyword")).toHaveText("fn");
	await expect(page.locator(".aster-editor-diagnostic-marker.error")).toHaveCount(1);

	const input = page.locator(".aster-editor-input");
	await input.focus();
	await page.keyboard.press("ControlOrMeta+Shift+o");
	await expect(page.locator(".aster-editor-goto-symbol-item")).toHaveText("main");
});

test("text-model editor has the accessibility contract", async ({ page }) => {
	await page.goto("/textModel.html");
	const editor = page.locator(".aster-editor");
	const input = page.locator(".aster-editor-input");
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
