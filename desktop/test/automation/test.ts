import { test as base, type ElectronApplication, type Page } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { launchElectron } from "./electron.js";
import type { PlaywrightDriver } from "./playwrightDriver.js";

interface ElectronFixtures {
  readonly application: ElectronApplication;
  readonly driver: PlaywrightDriver;
  readonly workbenchPage: Page;
}

export const test = base.extend<ElectronFixtures>({
  application: async ({}, use) => {
    const userDataDirectory = await mkdtemp(join(tmpdir(), "zeta-playwright-"));
    const { application } = await launchElectron({ userDataDirectory });
    try {
      await use(application);
    } finally {
      await application.close().catch(() => undefined);
      await rm(userDataDirectory, { force: true, recursive: true });
    }
  },
  driver: async ({ application }, use) => {
    const page = application.windows()[0] ?? await application.waitForEvent("window", { timeout: 30_000 });
    const { PlaywrightDriver } = await import("./playwrightDriver.js");
    const driver = new PlaywrightDriver(application, page);
    await driver.waitForWorkbench();
    await use(driver);
  },
  workbenchPage: async ({ driver }, use, testInfo) => {
    const page = driver.currentPage;
    const pageErrors: string[] = [];
    page.on("pageerror", error => pageErrors.push(error.stack ?? error.message));
    await page.context().tracing.start({ screenshots: true, snapshots: true, sources: true });
    await use(page);

    const failed = testInfo.status !== testInfo.expectedStatus;
    if (failed && !page.isClosed()) {
      const screenshotPath = testInfo.outputPath("workbench.png");
      await page.screenshot({ path: screenshotPath });
      await testInfo.attach("workbench", { path: screenshotPath, contentType: "image/png" });
    }
    if (failed) {
      const tracePath = testInfo.outputPath("trace.zip");
      await page.context().tracing.stop({ path: tracePath });
      await testInfo.attach("trace", { path: tracePath, contentType: "application/zip" });
    } else {
      await page.context().tracing.stop();
    }
    if (pageErrors.length > 0 && !failed) {
      throw new Error(`Workbench page errors:\n${pageErrors.join("\n\n")}`);
    }
  },
});

export { expect } from "@playwright/test";
