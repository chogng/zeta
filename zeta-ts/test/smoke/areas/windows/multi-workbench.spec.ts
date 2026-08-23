import type { ElectronApplication, Page } from "@playwright/test";
import { realpath } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { expect, test } from "../../../automation/test.js";
import { createTestWorkspace, disposeTestWorkspace } from "../../../automation/testWorkspace.js";
import { Workbench } from "../../../automation/workbench.js";

test("a second instance opens an independent Workbench and reuses an existing Workspace window", async ({ application, target, testWorkspace, workbench }) => {
  test.skip(target.kind !== "electron", "This scenario verifies the Electron Workbench window registry.");
  if (target.kind !== "electron" || !("windows" in application)) return;

  const secondWorkspace = await createTestWorkspace();
  let secondPage: Page | undefined;
  try {
    const secondPagePromise = application.waitForEvent("window");
    await emitSecondInstance(application, secondWorkspace.directory);
    secondPage = await secondPagePromise;
    const secondWorkbench = new Workbench(secondPage);
    await secondWorkbench.waitForReady();

    await expect.poll(() => application.windows().length).toBe(2);
    expect(await canonicalWorkspacePath(workbench.page)).toBe(await realpath(testWorkspace.directory));
    expect(await canonicalWorkspacePath(secondPage)).toBe(await realpath(secondWorkspace.directory));

    await workbench.page.evaluate(() => { document.title = "multi-workbench:first"; });
    await secondPage.evaluate(() => { document.title = "multi-workbench:second"; });
    await emitSecondInstance(application, testWorkspace.directory);

    await expect.poll(() => application.windows().length).toBe(2);
    await expect.poll(() => focusedWindowTitle(application)).toBe("multi-workbench:first");

    const closed = secondPage.waitForEvent("close");
    await secondPage.close();
    await closed;
    await expect.poll(() => application.windows().length).toBe(1);
    await expect(workbench.element).toBeVisible();
  } finally {
    if (secondPage && !secondPage.isClosed()) await secondPage.close().catch(() => undefined);
    await disposeTestWorkspace(secondWorkspace);
  }
});

async function emitSecondInstance(application: ElectronApplication, workspaceDirectory: string): Promise<void> {
  await application.evaluate(({ app }, directory) => {
    app.emit("second-instance", {} as never, [process.execPath, app.getAppPath(), "--folder", directory], process.cwd(), {});
  }, workspaceDirectory);
}

async function canonicalWorkspacePath(page: Page): Promise<string> {
  const uri = await page.evaluate(async () => {
    const bridge = (globalThis as unknown as { zeta?: { ipcRenderer?: { invoke(channel: string): Promise<unknown> } } }).zeta?.ipcRenderer;
    if (!bridge) throw new Error("Zeta renderer IPC bridge is unavailable");
    const value = await bridge.invoke("zeta:workspace:context:read");
    const candidate = value as { uri?: unknown };
    if (typeof candidate.uri !== "string") throw new Error("Workbench does not contain a folder Workspace");
    return candidate.uri;
  });
  return realpath(fileURLToPath(uri));
}

async function focusedWindowTitle(application: ElectronApplication): Promise<string | undefined> {
  return application.evaluate(({ BrowserWindow }) => BrowserWindow.getFocusedWindow()?.getTitle());
}
