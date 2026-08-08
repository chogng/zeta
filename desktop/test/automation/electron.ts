import { resolve } from "node:path";
import type { AppServerTestMode, DesktopProduct } from "./testTarget.js";

const desktopDirectory = resolve(import.meta.dirname, "../..");

export interface ElectronLaunchOptions {
  readonly appServerMode: AppServerTestMode;
  readonly userDataDirectory: string;
  readonly workspaceDirectory?: string;
  readonly product?: DesktopProduct;
}

export interface ElectronConfiguration {
  readonly args: readonly string[];
  readonly cwd: string;
  readonly env: Readonly<Record<string, string>>;
}

/** Resolves the Electron executable, arguments, and environment for a test run. */
export function resolveElectronConfiguration(options: ElectronLaunchOptions): ElectronConfiguration {
  const environment = Object.fromEntries(
    Object.entries(process.env).filter((entry): entry is [string, string] => entry[1] !== undefined),
  );
  if (options.appServerMode === "disabled") {
    environment.ZETA_DESKTOP_UI_ONLY = "1";
  } else {
    delete environment.ZETA_DESKTOP_UI_ONLY;
  }
  environment.ZETA_PRODUCT = options.product ?? "code";
  environment.ZETA_ELECTRON_MAIN = environment.ZETA_PRODUCT;
  delete environment.ELECTRON_RUN_AS_NODE;

  return {
    args: [
      "--disable-gpu",
      "--in-process-gpu",
      desktopDirectory,
      `--user-data-dir=${options.userDataDirectory}`,
      ...(options.workspaceDirectory === undefined ? [] : [`--folder=${options.workspaceDirectory}`]),
    ],
    cwd: desktopDirectory,
    env: environment,
  };
}
