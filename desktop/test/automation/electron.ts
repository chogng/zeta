import { createRequire } from "node:module";
import { resolve } from "node:path";
import type { AppServerTestMode } from "./testTarget.js";

const desktopDirectory = resolve(import.meta.dirname, "../..");
const require = createRequire(import.meta.url);

export interface ElectronLaunchOptions {
  readonly appServerMode: AppServerTestMode;
  readonly userDataDirectory: string;
  readonly product?: "academic" | "code" | "complete";
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
    executablePath: require("electron") as string,
    args: [desktopDirectory, `--user-data-dir=${options.userDataDirectory}`],
    cwd: desktopDirectory,
    env: environment,
  };
}
