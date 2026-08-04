import { test as base } from "@playwright/test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { launchBrowser } from "./playwrightBrowser.js";
import { launchElectron } from "./playwrightElectron.js";
import type { PlaywrightApplication, PlaywrightDriver } from "./playwrightDriver.js";
import { playwrightTargetForProject, type PlaywrightTarget } from "./testTarget.js";
import type { Workbench } from "./workbench.js";

interface PlaywrightFixtures {
  readonly target: PlaywrightTarget;
  readonly application: PlaywrightApplication;
  readonly driver: PlaywrightDriver;
  readonly workbench: Workbench;
}

export const test = base.extend<PlaywrightFixtures>({
  target: async ({ baseURL }, use, testInfo) => {
    await use(playwrightTargetForProject(testInfo.project.name, baseURL));
  },
  driver: async ({ target }, use) => {
    if (target.kind === "browser") {
      const { application, driver } = await launchBrowser(target);
      try {
        await use(driver);
      } finally {
        await application.close().catch(() => undefined);
      }
      return;
    }

    const userDataDirectory = await mkdtemp(join(tmpdir(), "zeta-playwright-"));
    const { application, driver } = await launchElectron({ appServerMode: target.appServerMode, userDataDirectory });
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
