import type { ElectronApplication, Page } from "@playwright/test";
import { PlaywrightDriver, type WindowSize } from "./playwrightDriver.js";

/** Adds Electron-hosted window control to the shared Workbench driver. */
export class ElectronPlaywrightDriver extends PlaywrightDriver {
  constructor(application: ElectronApplication, currentPage: Page) {
    super(application, currentPage);
  }

  override async setWindowSize(size: WindowSize): Promise<void> {
    const application = this.application;
    if (!("windows" in application)) {
      throw new Error("Electron window control requires an Electron application");
    }
    await application.evaluate(({ BrowserWindow }, requestedSize) => {
      const window = BrowserWindow.getAllWindows()[0];
      if (!window) throw new Error("Zeta workbench window is unavailable");
      window.setSize(requestedSize.width, requestedSize.height);
    }, size);
    await this.currentPage.waitForFunction(
      requestedSize => window.innerWidth === requestedSize.width && window.innerHeight === requestedSize.height,
      size,
    );
    await this.workbench.waitForUiIdle();
  }
}
