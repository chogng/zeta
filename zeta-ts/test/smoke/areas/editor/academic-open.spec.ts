import { readFile } from "node:fs/promises";
import { expect, test } from "../../../automation/test.js";

test("Academic opens the document engine and saves its structured document through the Workbench", async ({ target, testWorkspace, workbench }) => {
	test.skip(
		target.kind !== "electron" || target.appServerMode !== "required" || target.product !== "academic",
		"This scenario requires the Academic Electron App Server product",
	);

	const page = workbench.page;
	const explorer = page.locator(".zeta-explorer");
	await expect(explorer).toBeVisible();

	const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "paper.zeta-academic" });
	await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "academic workspace file appears in Explorer" }).toBe(1);
	await fileRow.click();

	const group = workbench.editors.groupAt(0);
	await expect(group.tabs).toHaveCount(1);
	await expect(group.tabs.first()).toContainText("paper.zeta-academic");
	await expect(group.content.locator(".zeta-structured-editor-pane")).toBeVisible();

	const formatting = group.content.locator(".zeta-structured-format-toolbar");
	const structuredInput = group.content.locator("textarea.zeta-document-text-input").first();
	const fontSize = formatting.locator("select[aria-label='Font size']");
	await expect(formatting).toHaveAttribute("data-context", "text");
	await expect(fontSize).toBeVisible();
	await structuredInput.evaluate(element => {
		const textarea = element as HTMLTextAreaElement;
		textarea.focus();
		textarea.setSelectionRange(0, textarea.value.length);
		textarea.dispatchEvent(new Event("select", { bubbles: true }));
	});
	await fontSize.selectOption("18");
	await expect(fontSize).toHaveValue("18");

	const input = group.content.locator(".zeta-document-text-block .aster-editor-input");
	await expect(input).toBeAttached();
	await input.focus();
	await input.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
	await input.type("const paper = 2;");
	await input.press(process.platform === "darwin" ? "Meta+S" : "Control+S");

	await expect.poll(
		() => readFile(testWorkspace.academicFile, "utf8"),
		{ timeout: 15_000, message: "Document engine save reaches the App Server workspace" },
	).toContain("\"fontSize\":18");
	await expect.poll(
		() => readFile(testWorkspace.academicFile, "utf8"),
		{ timeout: 15_000, message: "Document engine code textBlock save reaches the App Server workspace" },
	).toContain("const paper = 2;");
});
