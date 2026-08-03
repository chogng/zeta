import type { ElectronApplication, Page } from "@playwright/test";
import { Workbench } from "./workbench.js";

export interface WindowSize {
  readonly width: number;
  readonly height: number;
}

/** Small workbench-facing driver shared by Electron end-to-end tests. */
export class PlaywrightDriver {
  readonly workbench: Workbench;

  constructor(
    readonly application: ElectronApplication,
    readonly currentPage: Page,
  ) {
    this.workbench = new Workbench(currentPage);
  }

  async setWindowSize(size: WindowSize): Promise<void> {
    await this.application.evaluate(({ BrowserWindow }, requestedSize) => {
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
