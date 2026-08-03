import { test as base, type ElectronApplication } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type AppServerTestMode, launchElectron } from "./electron.js";
import type { PlaywrightDriver } from "./playwrightDriver.js";
import type { Workbench } from "./workbench.js";

interface ElectronFixtures {
  readonly appServerMode: AppServerTestMode;
  readonly application: ElectronApplication;
  readonly driver: PlaywrightDriver;
  readonly workbench: Workbench;
}

export const test = base.extend<ElectronFixtures>({
  appServerMode: async ({}, use, testInfo) => {
    await use(appServerModeForProject(testInfo.project.name));
  },
  driver: async ({ appServerMode }, use) => {
    const userDataDirectory = await mkdtemp(join(tmpdir(), "zeta-playwright-"));
    const { application, driver } = await launchElectron({ appServerMode, userDataDirectory });
    try {
      await use(driver);
    } finally {
      await application.close().catch(() => undefined);
      await rm(userDataDirectory, { force: true, recursive: true });
    }
  },
  application: async ({ driver }, use) => {
    await use(driver.application);
  },
  workbench: async ({ driver }, use, testInfo) => {
    const workbench = driver.workbench;
    const page = workbench.page;
    const pageErrors: string[] = [];
    page.on("pageerror", error => pageErrors.push(error.stack ?? error.message));
    await page.context().tracing.start({ screenshots: true, snapshots: true, sources: true });
    await use(workbench);

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

function appServerModeForProject(projectName: string): AppServerTestMode {
  switch (projectName) {
    case "ui":
      return "disabled";
    case "desktop":
      return "required";
    default:
      throw new Error(`Unsupported Playwright project: ${projectName}`);
  }
}
