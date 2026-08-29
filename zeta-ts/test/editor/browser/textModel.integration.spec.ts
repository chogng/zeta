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
	await expect(page.locator(".stanza-editor")).toBeVisible();
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.apiText)).toBe("editor-api");

	const input = page.locator(".stanza-editor-input");
	await input.focus();
	await page.keyboard.press("Control+Home");
	const caret = page.locator('.stanza-editor-caret.primary');
	await expect(caret).toHaveClass(/cursor-style-line/u);
	await page.keyboard.press('Insert');
	await expect(caret).toHaveClass(/cursor-style-block/u);
	await expect(caret).toHaveClass(/token-keyword/u);
	await expect(caret).toHaveText('f');
	await page.keyboard.press('Insert');
	await expect(caret).toHaveClass(/cursor-style-line/u);
	await page.keyboard.type("/* integrated */ ");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getValue())).toBe("/* integrated */ fn main() {\n  answer();\n}\n");

	await page.keyboard.press("ControlOrMeta+z");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getValue())).toBe("fn main() {\n  answer();\n}\n");
	await page.evaluate(() => window.zetaTextModelIntegration.save());
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getSavedText())).toBe("fn main() {\n  answer();\n}\n");
	await expect.poll(() => workers.length).toBeGreaterThan(0);
});

test("cursor layer retains nodes, animates stable moves, and resolves multi-cursor colors", async ({ page }) => {
	await page.goto("/textModel.html");
	await expect(page.locator(".stanza-editor")).toBeVisible();
	await expect(page.locator(".stanza-editor-token.token-keyword")).toHaveText("fn");
	const editor = page.locator(".stanza-editor");
	const layer = editor.locator(".stanza-editor-cursors-layer");
	const retainedCaret = layer.locator('.stanza-editor-caret[data-selection-index="0"]');
	await expect(layer).toHaveClass(/cursor-smooth-caret-animation/u);
	await retainedCaret.evaluate(element => { element.dataset.retainedIdentity = "true"; });

	const stableMove = await page.evaluate(() => {
		window.zetaTextModelIntegration.setCursors([{ lineIndex: 1, columnIndex: 2 }]);
		const caret = document.querySelector<HTMLElement>('.stanza-editor-caret[data-selection-index="0"]');
		if (!caret) throw new Error("Moved cursor is missing");
		return { top: caret.style.top, transitionProperty: caret.style.transitionProperty };
	});
	expect(stableMove).toEqual({ top: "20px", transitionProperty: "" });
	await expect(retainedCaret).toHaveAttribute("data-retained-identity", "true");

	await editor.evaluate(element => {
		element.style.setProperty("--zeta-editor-multi-cursor-primary-foreground", "#010203");
		element.style.setProperty("--zeta-editor-multi-cursor-secondary-foreground", "#040506");
	});
	const countChangeTransitions = await page.evaluate(() => {
		window.zetaTextModelIntegration.setCursors([
			{ lineIndex: 0, columnIndex: 0 },
			{ lineIndex: 1, columnIndex: 2 },
		], 1);
		return [...document.querySelectorAll<HTMLElement>(".stanza-editor-caret")]
			.map(caret => caret.style.transitionProperty);
	});
	expect(countChangeTransitions).toEqual(["none", "none"]);
	const primary = layer.locator(".stanza-editor-caret.cursor-primary");
	const secondary = layer.locator(".stanza-editor-caret.cursor-secondary");
	await expect(primary).toHaveCount(1);
	await expect(secondary).toHaveCount(1);
	await expect(primary).toHaveCSS("background-color", "rgb(1, 2, 3)");
	await expect(secondary).toHaveCSS("background-color", "rgb(4, 5, 6)");

	await page.locator(".stanza-editor-input").focus();
	await page.keyboard.press("Insert");
	await page.evaluate(() => {
		window.zetaTextModelIntegration.setValue("fn main() {\n  A👩‍🔧B answer();\n}\n");
		window.zetaTextModelIntegration.setCursors([{ lineIndex: 1, columnIndex: 4 }]);
	});
	await expect(retainedCaret).toHaveText("👩‍🔧");
	await page.keyboard.press("Insert");

	await page.evaluate(() => {
		window.zetaTextModelIntegration.setValue(Array.from({ length: 60 }, (_, index) => `fn main() { answer(); } // ${index}`).join("\n"));
		window.zetaTextModelIntegration.setCursors([{ lineIndex: 59, columnIndex: 0 }]);
	});
	await expect(retainedCaret).toHaveAttribute("data-retained-identity", "true");
	await expect(retainedCaret).toHaveCSS("display", "none");
	await page.evaluate(() => window.zetaTextModelIntegration.revealPosition(59, 0));
	await expect(retainedCaret).toHaveCSS("display", "block");
	await expect(retainedCaret).toHaveAttribute("data-retained-identity", "true");
});

test("text-model editor projects revision-bound Rust syntax, diagnostics, folding, and symbols", async ({ page }) => {
	await page.goto("/textModel.html");
	await expect.poll(() => page.evaluate(() => window.zetaTextModelIntegration.getSyntaxAnalysisCount())).toBeGreaterThan(0);
	await expect(page.locator(".stanza-editor-token.token-keyword")).toHaveText("fn");
	await expect(page.locator(".stanza-editor-diagnostic-marker.error")).toHaveCount(1);
	const symbolIcon = page.locator(".stanza-editor-symbol-icon");
	await expect(symbolIcon).toHaveCount(1);
	await expect(symbolIcon).toHaveAttribute("data-decoration-owner", "symbol-icons");
	await expect(symbolIcon.locator("xpath=..")).toHaveClass(/stanza-editor-line-lines-decorations/u);

	const input = page.locator(".stanza-editor-input");
	await input.focus();
	await page.keyboard.press("ControlOrMeta+Shift+o");
	await expect(page.locator(".stanza-editor-goto-symbol-item")).toHaveText("main");
});

test("short documents have no false scroll range and use a proportional hover slider", async ({ page }) => {
	await page.goto("/textModel.html");
	await expect(page.locator(".stanza-editor")).toBeVisible();
	const geometry = await page.locator(".stanza-editor").evaluate(editor => {
		const minimap = editor.querySelector<HTMLElement>(".stanza-editor-minimap");
		const slider = editor.querySelector<HTMLElement>(".stanza-editor-minimap-slider");
		if (!minimap || !slider) throw new Error("Missing minimap geometry");
		return {
			clientHeight: editor.clientHeight,
			scrollHeight: editor.scrollHeight,
			scrollTop: editor.scrollTop,
			sliderHidden: slider.hidden,
			sliderHeight: slider.getBoundingClientRect().height,
			minimapHeight: minimap.getBoundingClientRect().height,
		};
	});

	expect(geometry.scrollHeight).toBe(geometry.clientHeight);
	expect(geometry.scrollTop).toBe(0);
	expect(geometry.sliderHidden).toBe(false);
	expect(geometry.sliderHeight).toBeLessThan(geometry.minimapHeight);
	const minimap = page.locator('.stanza-editor-minimap');
	const slider = page.locator('.stanza-editor-minimap-slider');
	await expect(slider).toHaveCSS('opacity', '0');
	await minimap.hover();
	await expect(slider).toHaveCSS('opacity', '1');
});

test("glyph margin, line numbers, and folding controls keep VS Code gutter order", async ({ page }) => {
	await page.goto("/textModel.html");
	const glyphMargin = page.locator(".stanza-editor-glyph-margin");
	const foldingControl = page.locator(".stanza-editor-fold-toggle[data-logical-line-index='0']");
	await expect(glyphMargin).toBeVisible();
	await expect(foldingControl).toBeVisible();
	const firstLine = page.locator(".stanza-editor-line[data-logical-line-index='0']");
	await expect(page.locator('.stanza-editor-lines')).toHaveCSS('cursor', 'text');
	const firstLineNumber = page.locator(".stanza-editor-line-margin[data-line-index='0'] .stanza-editor-line-number");
	await expect(firstLineNumber).toHaveText("1");
	const foldingBox = await foldingControl.boundingBox();
	const glyphMarginBox = await glyphMargin.boundingBox();
	const lineNumberBox = await firstLineNumber.boundingBox();
	const textBox = await firstLine.locator(".stanza-editor-line-text").boundingBox();
	assertBox(foldingBox, "folding control");
	assertBox(glyphMarginBox, "glyph margin");
	assertBox(lineNumberBox, "line number");
	assertBox(textBox, "line text");

	expect(glyphMarginBox.x + glyphMarginBox.width).toBe(lineNumberBox.x);
	expect(lineNumberBox.x + lineNumberBox.width).toBeLessThanOrEqual(foldingBox.x);
	expect(foldingBox.x + foldingBox.width).toBeLessThanOrEqual(textBox.x);

	const editor = page.locator(".stanza-editor");
	const input = page.locator(".stanza-editor-input");
	await input.focus();
	await page.keyboard.press("Control+Home");
	await page.keyboard.type("x".repeat(200));
	await expect.poll(() => editor.evaluate(element => element.scrollWidth - element.clientWidth)).toBeGreaterThan(0);
	await editor.evaluate(element => {
		element.scrollLeft = 160;
		element.dispatchEvent(new Event("scroll"));
	});
	await expect.poll(() => editor.evaluate(element => element.scrollLeft)).toBeGreaterThan(0);
	const editorBox = await editor.boundingBox();
	const scrolledGlyphMarginBox = await glyphMargin.boundingBox();
	const scrolledFoldingBox = await foldingControl.boundingBox();
	const scrolledLineNumberBox = await firstLineNumber.boundingBox();
	assertBox(editorBox, "editor");
	assertBox(scrolledGlyphMarginBox, "scrolled glyph margin");
	assertBox(scrolledFoldingBox, "scrolled folding control");
	assertBox(scrolledLineNumberBox, "scrolled line number");

	expect(scrolledGlyphMarginBox.x).toBe(editorBox.x);
	expect(scrolledGlyphMarginBox.x + scrolledGlyphMarginBox.width).toBe(scrolledLineNumberBox.x);
	expect(scrolledLineNumberBox.x + scrolledLineNumberBox.width).toBe(scrolledFoldingBox.x);
});

test("text-model editor has the accessibility contract", async ({ page }) => {
	await page.goto("/textModel.html");
	const editor = page.locator(".stanza-editor");
	const input = page.locator(".stanza-editor-input");
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

function assertBox(box: { readonly x: number; readonly y: number; readonly width: number; readonly height: number } | null, name: string): asserts box is { readonly x: number; readonly y: number; readonly width: number; readonly height: number } {
	expect(box, `Expected ${name} geometry`).not.toBeNull();
}
