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
		window.zetaDocumentModelIntegration?.dispose();
	}).catch(() => undefined);
	expect(pageErrors.get(page) ?? []).toEqual([]);
});

test("document-model editor API, structured editing, and embedded code block run in real browsers", async ({ page }) => {
	await page.goto("/documentModel.html");
	await expect(page.locator("#code-block .stanza-editor")).toBeVisible();
	await expect(page.locator("#code-block .stanza-structured-format-toolbar")).toHaveAttribute("data-context", "code");
	await expect(page.locator("#code-block .stanza-structured-format-code-context")).toHaveText("Code block · Academic");
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.apiDocumentType)).toBe("doc");

	const codeBlockInput = page.locator("#code-block .stanza-editor-input");
	await codeBlockInput.focus();
	await page.keyboard.press("Control+Home");
	await page.keyboard.type("// bridge\n");
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getCodeBlockText())).toBe("// bridge\nconst editor = 1;");
	await page.evaluate(() => window.zetaDocumentModelIntegration.saveCodeBlock());
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getSavedCodeBlock())).toContain("// bridge\\nconst editor = 1;");

	const structuredInput = page.locator("#document-editor textarea.stanza-document-text-input").first();
	await structuredInput.focus();
	await page.keyboard.press("End");
	await page.keyboard.press("Enter");
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getStructuredBlockTexts())).toEqual(["Title", "", "Body"]);
	await page.keyboard.press("Control+z");
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getStructuredBlockTexts())).toEqual(["Title", "Body"]);
});

test("document-model editor persists selected font, size, and emphasis formatting", async ({ page }) => {
	await page.goto("/documentModel.html");
	const input = page.locator("#document-editor textarea.stanza-document-text-input").first();
	const fontFamily = page.locator("#document-editor select[aria-label='Font family']");
	const fontSize = page.locator("#document-editor select[aria-label='Font size']");
	await input.evaluate(element => {
		const textarea = element as HTMLTextAreaElement;
		textarea.focus();
		textarea.setSelectionRange(0, 5);
		textarea.dispatchEvent(new Event("select", { bubbles: true }));
	});
	await fontFamily.selectOption("serif");
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getStructuredFirstTextMarks())).toEqual([
		{ type: "textStyle", attrs: { fontFamily: "serif" } },
	]);
	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getStructuredSelection())).toMatchObject({ kind: "text", anchor: { offset: 0 }, head: { offset: 5 } });
	await fontSize.selectOption("18");
	await expect(fontSize).toHaveValue("18");
	const bold = page.locator("#document-editor [data-action-id='bold'] button");
	await bold.click();

	await expect.poll(() => page.evaluate(() => window.zetaDocumentModelIntegration.getStructuredFirstTextMarks())).toEqual([
		{ type: "textStyle", attrs: { fontFamily: "serif", fontSize: 18 } },
		{ type: "strong", attrs: {} },
	]);

	const styled = page.locator("#document-editor .stanza-document-mark-textStyle[data-font-family='serif']");
	await expect(styled).toHaveText("Title");
	await expect(styled).toHaveCSS("font-size", "18px");
	await expect(bold).toHaveAttribute("aria-pressed", "true");
});

test("document-model editor exposes collaboration as a separate contribution", async ({ page }) => {
	await page.goto("/documentModel.html");
	await page.evaluate(() => {
		const responses = ["", "editor-browser-room"];
		window.prompt = () => responses.shift() ?? null;
	});
	const toolbar = page.locator("#document-editor .stanza-document-collaboration-toolbar");
	const start = toolbar.locator("[data-action-id='startCollaboration'] button");
	await expect(toolbar).toHaveAttribute("data-state", "inactive");
	await start.click();
	await expect(toolbar).toHaveAttribute("data-state", "connected");
	await expect(toolbar.locator(".stanza-document-collaboration-status")).toHaveText("Room: editor-browser-room");
});

test("document-model editor exposes remote-owner invitations", async ({ page }) => {
	await page.goto("/documentModel.html");
	const prompts = ["https://collaboration.zeta.example", "0123456789abcdef0123456789abcdef", "editor-browser-room", "Writer", "viewer"];
	await page.evaluate(({ prompts }) => {
		window.prompt = () => prompts.shift() ?? null;
	}, { prompts });
	const toolbar = page.locator("#document-editor .stanza-document-collaboration-toolbar");
	await toolbar.locator("[data-action-id='startCollaboration'] button").click();
	await expect(toolbar.locator("[data-action-id='inviteCollaborator'] button")).toBeVisible();
	await toolbar.locator("[data-action-id='inviteCollaborator'] button").click();
	await expect(toolbar.locator(".stanza-document-collaboration-status")).toContainText("Invitation created for Writer");
	await expect(toolbar.locator(".stanza-document-collaboration-invitation-token")).toHaveText("Room ID: editor-browser-room\nAccess token: editor-browser-member-token");
	await toolbar.locator("[data-action-id='manageCollaborators'] button").click();
	await expect(toolbar.locator("[data-principal-id='browser-member']")).toContainText("Writereditor · browser-member");
	await toolbar.locator("[data-principal-id='browser-member'] button").first().click();
	await expect(toolbar.locator(".stanza-document-collaboration-invitation-token")).toHaveText("Room ID: editor-browser-room\nAccess token: editor-browser-rotated-token");
});

test("document-model editor has the accessibility contract", async ({ page }) => {
	await page.goto("/documentModel.html");
	const toolbar = page.locator("#document-editor .stanza-structured-format-toolbar");
	const structuredInput = page.locator("#document-editor textarea.stanza-document-text-input").first();
	await expect(toolbar).toHaveAttribute("role", "group");
	await expect(toolbar).toHaveAttribute("aria-label", "Document formatting");
	await expect(toolbar.locator(".stanza-structured-format-inline-actions")).toHaveAttribute("role", "toolbar");
	await expect(toolbar.locator(".stanza-structured-format-document-actions button")).toHaveCount(13);
	await expect(toolbar.locator("select[aria-label='Font family']")).toBeVisible();
	await expect(toolbar.locator("select[aria-label='Font size']")).toBeVisible();
	await expect(structuredInput).toBeVisible();

	await injectAxe(page);
	const accessibility = await getAxeResults(page, undefined, { runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "best-practice"] } });
	expect(accessibility.violations.filter(violation => violation.impact === "critical")).toEqual([]);
	const contrast = await getAxeResults(page, undefined, { runOnly: { type: "rule", values: ["color-contrast"] } });
	expect(contrast.violations).toEqual([]);
});
