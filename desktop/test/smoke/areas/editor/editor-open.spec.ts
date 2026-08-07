import { readFile } from "node:fs/promises";
import { expect, test } from "../../../automation/test.js";

test("App Server workspace files open in Alpha and save through the editor region", async ({ target, testWorkspace, workbench }) => {
  test.skip(
    target.kind !== "electron" || target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code Electron App Server product",
  );

  const page = workbench.page;
  const explorer = page.locator(".zeta-explorer");
  await expect(explorer).toBeVisible();

  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.ts" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  await expect(group.tabs).toHaveCount(1);
  await expect(group.tabs.first()).toContainText("main.ts");
  await expect(group.content.locator(".zeta-alpha-editor")).toBeVisible();

  const input = group.content.locator(".zeta-alpha-editor-input");
  await expect(input).toBeAttached();
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
  await input.type("const value = 2;");
  await input.press(process.platform === "darwin" ? "Meta+S" : "Control+S");

  await expect.poll(
    () => readFile(testWorkspace.file, "utf8"),
    { timeout: 15_000, message: "Alpha save reaches the App Server workspace" },
  ).toBe("const value = 2;");
});

test("Code consumes App Server Rust syntax facts in Alpha", async ({ target, workbench }) => {
  test.skip(
    target.kind !== "electron" || target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code Electron App Server product",
  );

  const page = workbench.page;
  const explorer = page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  await expect(group.content.locator(".zeta-alpha-editor")).toBeVisible();
  await expect(group.content.locator(".zeta-alpha-editor-token.token-keyword")).toContainText("fn");

  const input = group.content.locator(".zeta-alpha-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+Shift+O" : "Control+Shift+O");
  await expect(group.content.locator(".zeta-alpha-editor-goto-symbol-item")).toContainText("main");
});
