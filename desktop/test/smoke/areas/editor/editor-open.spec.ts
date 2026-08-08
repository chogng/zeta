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

test("Code renders workspace PDFs and persists review annotations", async ({ target, testWorkspace, workbench }) => {
  test.skip(
    target.kind !== "electron" || target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code Electron App Server product",
  );

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "paper.pdf" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "PDF appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  await expect(group.tabs).toHaveCount(1);
  await expect(group.tabs.first()).toContainText("paper.pdf");
  const reader = group.content.locator(".zeta-pdf-editor");
  await expect(reader).toBeVisible();
  await expect(reader.locator(".zeta-pdf-page-canvas")).toBeVisible();

  await reader.locator("[data-action-id='zeta.pdf.annotations.highlight'] button").click();
  const layer = reader.locator(".zeta-pdf-annotation-layer");
  const bounds = await layer.boundingBox();
  if (!bounds) throw new Error("PDF annotation layer has no visual bounds");
  await workbench.page.mouse.move(bounds.x + 24, bounds.y + 24);
  await workbench.page.mouse.down();
  await workbench.page.mouse.move(bounds.x + Math.min(180, bounds.width - 12), bounds.y + Math.min(90, bounds.height - 12));
  await workbench.page.mouse.up();

  await expect(reader.locator(".zeta-pdf-annotation-highlight")).toHaveCount(1);
  await expect(reader.locator(".zeta-pdf-annotation-status")).toContainText("Unsaved annotations");
  await reader.locator("[data-action-id='zeta.pdf.annotations.save'] button").click();
  await expect.poll(
    async () => {
      try {
        const document = JSON.parse(await readFile(`${testWorkspace.pdfFile}.zeta-annotations.json`, "utf8")) as { annotations: unknown[] };
        return document.annotations.length;
      } catch {
        return 0;
      }
    },
    { timeout: 15_000, message: "PDF annotation sidecar is saved through the App Server workspace" },
  ).toBe(1);
});
