import { resolve } from "node:path";
import { createRequire } from "node:module";
import type { AppServerTestMode, DesktopProduct } from "./testTarget.js";

const desktopDirectory = resolve(import.meta.dirname, "../..");
const electronExecutablePath = createRequire(import.meta.url)("electron") as string;

export interface ElectronLaunchOptions {
  readonly appServerMode: AppServerTestMode;
  readonly userDataDirectory: string;
  readonly workspaceDirectory?: string;
  readonly product?: DesktopProduct;
}

export interface ElectronConfiguration {
  readonly executablePath: string;
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
  delete environment.ELECTRON_RUN_AS_NODE;

  return {
    executablePath: electronExecutablePath,
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
