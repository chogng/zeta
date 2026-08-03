import { _electron, type ElectronApplication } from "@playwright/test";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { PlaywrightDriver } from "./playwrightDriver.js";

const desktopDirectory = resolve(import.meta.dirname, "../..");
const require = createRequire(import.meta.url);

export interface ElectronLaunchOptions {
  readonly userDataDirectory: string;
  readonly product?: "academic" | "code" | "complete";
}

export interface ElectronLaunchResult {
  readonly application: ElectronApplication;
  readonly driver: PlaywrightDriver;
}

/** Launches the built Zeta Desktop application with an isolated test profile. */
export async function launchElectron(options: ElectronLaunchOptions): Promise<ElectronLaunchResult> {
  const environment = Object.fromEntries(
    Object.entries(process.env).filter((entry): entry is [string, string] => entry[1] !== undefined),
  );
  environment.ZETA_DESKTOP_UI_ONLY = "1";
  environment.ZETA_PRODUCT = options.product ?? "code";
  delete environment.ELECTRON_RUN_AS_NODE;

  const application = await _electron.launch({
    executablePath: require("electron") as string,
    args: [desktopDirectory, `--user-data-dir=${options.userDataDirectory}`],
    cwd: desktopDirectory,
    env: environment,
    timeout: 30_000,
  });
  const page = application.windows()[0] ?? await application.waitForEvent("window", { timeout: 30_000 });
  const driver = new PlaywrightDriver(application, page);
  await driver.workbench.waitForReady();
  return { application, driver };
}
