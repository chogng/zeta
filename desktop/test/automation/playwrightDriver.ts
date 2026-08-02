import type { ElectronApplication, Page } from "@playwright/test";

export interface WindowSize {
  readonly width: number;
  readonly height: number;
}

/** Small workbench-facing driver shared by Electron end-to-end tests. */
export class PlaywrightDriver {
  constructor(
    private readonly application: ElectronApplication,
    readonly currentPage: Page,
  ) {}

  async waitForWorkbench(): Promise<void> {
    await this.currentPage.locator(".zeta-workbench").waitFor({ state: "visible" });
    await this.currentPage.locator(".zeta-workbench-editor .zeta-editor-group").waitFor({ state: "visible" });
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
  }
}
