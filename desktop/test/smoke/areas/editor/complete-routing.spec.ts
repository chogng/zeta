import { expect, test } from "../../../automation/test.js";

test("Complete routes source files to Alpha and academic documents to Gama", async ({ target, workbench }) => {
  test.skip(
    target.kind !== "electron" || target.appServerMode !== "required" || target.product !== "complete",
    "This scenario requires the Complete Electron App Server product",
  );

  const page = workbench.page;
  const explorer = page.locator(".zeta-explorer");
  await expect(explorer).toBeVisible();

  const sourceRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.ts" });
  const academicRow = explorer.locator(".zeta-tree-row").filter({ hasText: "paper.zeta-academic" });
  await expect.poll(() => sourceRow.count(), { timeout: 15_000, message: "source workspace file appears in Explorer" }).toBe(1);
  await expect.poll(() => academicRow.count(), { timeout: 15_000, message: "academic workspace file appears in Explorer" }).toBe(1);

  const group = workbench.editors.groupAt(0);
  await sourceRow.click();
  await expect(group.content.locator(".zeta-editor-pane-host:not([hidden]) .zeta-alpha-editor[aria-label='main.ts']")).toBeVisible();

  await academicRow.click();
  await expect(group.tabs).toHaveCount(2);
  await expect(group.content.locator(".zeta-editor-pane-host:not([hidden]) .zeta-structured-editor-pane")).toBeVisible();

  await sourceRow.click();
  await expect(group.content.locator(".zeta-editor-pane-host:not([hidden]) .zeta-alpha-editor[aria-label='main.ts']")).toBeVisible();
});
