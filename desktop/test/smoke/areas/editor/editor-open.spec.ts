import { readFile } from "node:fs/promises";
import type { Page } from "@playwright/test";
import { expect, test } from "../../../automation/test.js";

test("App Server workspace files open in Aster and save through the editor region", async ({ target, testWorkspace, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code App Server product",
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
  await expect(group.content.locator(".aster-editor")).toBeVisible();

  const input = group.content.locator(".aster-editor-input");
  await expect(input).toBeAttached();
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
  await input.type("const value = 2;");
  await input.press(process.platform === "darwin" ? "Meta+S" : "Control+S");

  await expect.poll(
    () => readFile(testWorkspace.file, "utf8"),
    { timeout: 15_000, message: "Aster save reaches the App Server workspace" },
  ).toBe("const value = 2;");
});

test("Code consumes App Server Rust syntax facts in Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code App Server product",
  );

  const page = workbench.page;
  const explorer = page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  await expect(group.content.locator(".aster-editor")).toBeVisible();
  await expect(group.content.locator(".aster-editor-token.token-keyword").filter({ hasText: "fn" }).first()).toBeVisible();

  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+Shift+O" : "Control+Shift+O");
  await expect(group.content.locator(".aster-editor-goto-symbol-item")).toContainText("main");
});

test("Code shows App Server LSP completions in Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+End" : "Control+End");
  await input.press("ArrowUp");
  await input.press("ArrowUp");
  await input.press("End");
  await input.press(process.platform === "darwin" ? "Meta+Space" : "Control+Space");

  const options = group.content.locator(".aster-editor-completion-option");
  await expect.poll(() => options.count(), { timeout: 60_000, message: "LSP completion candidates appear" }).toBeGreaterThan(0);
  await expect(options.filter({ hasText: "len" }).first()).toBeVisible();
});

test("Code streams current App Server LSP diagnostics into Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const marker = group.content.locator(".aster-editor-diagnostic-marker.error[title*='fixture diagnostic']");
  await expect(group.content.locator(".aster-editor-diagnostic-marker.error[title*='fixture diagnostic v1']")).toBeVisible({ timeout: 60_000 });

  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+End" : "Control+End");
  await input.type(" ");
  await expect(group.content.locator(".aster-editor-diagnostic-marker.error[title*='fixture diagnostic v2']")).toBeVisible({ timeout: 60_000 });
  await expect(marker).toHaveCount(1);
});

test("Code applies and undoes App Server LSP document formatting in Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const secondLine = group.content.locator(".aster-editor-line-text").nth(1);
  await expect(secondLine).toHaveText(/^ {4}let /);
  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+Shift+I" : "Control+Shift+I");
  await expect(secondLine).toHaveText(/^ {2}let /, { timeout: 60_000 });
  await input.press(process.platform === "darwin" ? "Meta+Z" : "Control+Z");
  await expect(secondLine).toHaveText(/^ {4}let /);
});

test("Code shows App Server LSP parameter hints in Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+Shift+Space" : "Control+Shift+Space");
  await expect(group.content.locator(".aster-editor-parameter-hints")).toContainText("String::from(value: &str)", { timeout: 60_000 });
  await expect(group.content.locator(".aster-editor-parameter-hints-parameter.active")).toHaveText("value: &str");
  await input.press("Escape");
  await input.press(process.platform === "darwin" ? "Meta+End" : "Control+End");
  await input.type("(");
  await expect(group.content.locator(".aster-editor-parameter-hints")).toContainText("String::from(value: &str)", { timeout: 60_000 });
});

test("Code shows App Server LSP inlay hints in Aster", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const hint = workbench.editors.groupAt(0).content.locator(".aster-editor-inlay-hint").filter({ hasText: ": String" });
  await expect(hint).toBeVisible({ timeout: 60_000 });
  await expect(hint).toHaveAttribute("title", "inferred type");
});

test("Code keeps App Server LSP linked edits in one undo step", async ({ target, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code" || !process.env.ZETA_PLAYWRIGHT_LANGUAGE_SERVER,
    "This scenario requires Code with the smoke-test language server",
  );
  test.setTimeout(120_000);

  const explorer = workbench.page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.rs" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "Rust workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const editor = group.content.locator(".aster-editor");
  const input = editor.locator(".aster-editor-input");
  await input.focus();
  await input.press(process.platform === "darwin" ? "Meta+Home" : "Control+Home");
  await input.press("ArrowDown");
  await input.press("Home");
  for (let index = 0; index < 9; index += 1) await input.press("ArrowRight");
  await expect(editor).toHaveClass(/linked-editing-active/, { timeout: 60_000 });

  await input.type("X");
  await expect(group.content.locator(".aster-editor-line-text").nth(1)).toContainText("mXessage");
  await expect(group.content.locator(".aster-editor-line-text").nth(2)).toContainText("mXessage");
  await input.press(process.platform === "darwin" ? "Meta+Z" : "Control+Z");
  await expect(group.content.locator(".aster-editor-line-text").nth(1)).toContainText("let message");
  await expect(group.content.locator(".aster-editor-line-text").nth(2)).toContainText("message.");
});

test("Code renders workspace PDFs and persists review annotations", async ({ target, testWorkspace, workbench }) => {
  test.skip(
    target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the Code App Server product",
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

test.describe("large files", () => {
  test.use({ includeLargeTestFile: true });

  test("Code keeps a 300,001-line file editable and saveable without background tokenization", async ({ target, testWorkspace, workbench }) => {
    test.skip(
      target.appServerMode !== "required" || target.product !== "code",
      "This scenario requires the Code App Server product",
    );
    test.setTimeout(120_000);

    const explorer = workbench.page.locator(".zeta-explorer");
    const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "large.ts" });
    await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "large file appears in Explorer" }).toBe(1);
    await fileRow.click();

    const group = workbench.editors.groupAt(0);
    const editor = group.content.locator(".aster-editor");
    await expect(editor).toBeVisible({ timeout: 60_000 });
    await expect(editor.locator(".aster-editor-token")).toHaveCount(0);

    const input = editor.locator(".aster-editor-input");
    await input.focus();
    await input.press("ControlOrMeta+Home");
    await input.type("// edited\n");
    await input.press("ControlOrMeta+S");

    await expect.poll(
      async () => (await readFile(testWorkspace.largeFile, "utf8")).startsWith("// edited\nlet value = 1;"),
      { timeout: 60_000, message: "large-file edit reaches the App Server workspace" },
    ).toBe(true);
  });
});

test("Code restores unsaved editor content after a browser reload", async ({ target, testWorkspace, workbench }) => {
  test.skip(
    target.kind !== "browser" || target.appServerMode !== "required" || target.product !== "code",
    "This scenario requires the browser-hosted Code App Server product",
  );

  const page = workbench.page;
  const explorer = page.locator(".zeta-explorer");
  const fileRow = explorer.locator(".zeta-tree-row").filter({ hasText: "main.ts" });
  await expect.poll(() => fileRow.count(), { timeout: 15_000, message: "workspace file appears in Explorer" }).toBe(1);
  await fileRow.click();

  const group = workbench.editors.groupAt(0);
  const input = group.content.locator(".aster-editor-input");
  await input.focus();
  await input.press("ControlOrMeta+A");
  await input.type("const recovered = 42;");
  await expect.poll(() => hasWorkingCopyBackup(page, "const recovered = 42;"), { message: "dirty editor content reaches IndexedDB" }).toBe(true);
  expect(await readFile(testWorkspace.file, "utf8")).toBe("const value = 1;\n");

  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.locator(".zeta-workbench")).toBeVisible();
  await expect(group.tabs.filter({ hasText: "main.ts" })).toHaveCount(1);
  await expect(group.content.locator(".aster-editor-line-text").first()).toContainText("const recovered = 42;");
  expect(await readFile(testWorkspace.file, "utf8")).toBe("const value = 1;\n");

  const restoredInput = group.content.locator(".aster-editor-input");
  await restoredInput.focus();
  await restoredInput.press("ControlOrMeta+S");
  await expect.poll(() => readFile(testWorkspace.file, "utf8"), { message: "restored content can still be saved" }).toBe("const recovered = 42;");
  await expect.poll(() => hasWorkingCopyBackup(page, "const recovered = 42;"), { message: "saving removes the crash backup" }).toBe(false);
});

async function hasWorkingCopyBackup(page: Page, content: string): Promise<boolean> {
  return page.evaluate(async expectedContent => {
    const database = await new Promise<IDBDatabase>((resolve, reject) => {
      const opening = indexedDB.open("zeta-working-copy-backups", 1);
      opening.onsuccess = () => resolve(opening.result);
      opening.onerror = () => reject(opening.error ?? new Error("Could not inspect working-copy backups"));
    });
    try {
      const records = await new Promise<Array<{ readonly content?: string }>>((resolve, reject) => {
        const request = database.transaction("backups", "readonly").objectStore("backups").getAll();
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error ?? new Error("Could not read working-copy backups"));
      });
      return records.some(record => record.content === expectedContent);
    } finally {
      database.close();
    }
  }, content);
}
