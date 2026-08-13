import { _electron, type ElectronApplication } from "@playwright/test";
import { ElectronPlaywrightDriver } from "./electronDriver.js";
import { resolveElectronConfiguration, type ElectronLaunchOptions } from "./electron.js";

export interface ElectronLaunchResult {
  readonly application: ElectronApplication;
  readonly driver: ElectronPlaywrightDriver;
}

/** Launches Zeta Desktop through Playwright's Electron adapter. */
export async function launchElectron(options: ElectronLaunchOptions): Promise<ElectronLaunchResult> {
  const configuration = resolveElectronConfiguration(options);
  const application = await _electron.launch({
    args: [...configuration.args],
    cwd: configuration.cwd,
    env: configuration.env,
    executablePath: configuration.executablePath,
    timeout: 30_000,
  });
  const page = application.windows()[0] ?? await application.waitForEvent("window", { timeout: 30_000 });
  const driver = new ElectronPlaywrightDriver(application, page);
  await driver.workbench.waitForReady();
  return { application, driver };
}
