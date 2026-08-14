import { app } from "electron/main";
import { join } from "node:path";
import type { ProductConfiguration } from "../../product/common/product.js";
import { resolveProductDataPaths } from "../../product/node/product.js";
import { type AppServerStartupMode, type ElectronMainIpcRouteContribution, ZetaApplication } from "./app.js";

/** Starts one explicitly selected Electron product after process bootstrap. */
export function startElectronApplication(product: ProductConfiguration, ipcRouteContributions: readonly ElectronMainIpcRouteContribution[] = []): void {
  const rendererRoot = join(app.getAppPath(), "dist", "renderer");
  const appServerStartupMode: AppServerStartupMode = process.env.ZETA_DESKTOP_UI_ONLY === "1"
    ? "disabled"
    : "required";

  app.setName(product.name);
  configureProductDataPaths(product);

  if (!app.requestSingleInstanceLock()) {
    app.quit();
    return;
  }

  const application = ZetaApplication.create({
    product,
    rendererRoot,
    appServerStartupMode,
    ipcRouteContributions,
  });

  app.on("second-instance", (_event, arguments_, cwd) => application.handleSecondInstance(arguments_, cwd));
  app.on("activate", () => application.handleActivate());
  app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit();
  });
  app.once("ready", () => {
    void startup(application);
  });
}

async function startup(application: ZetaApplication): Promise<void> {
  try {
    await application.startupAfterReady();
  } catch (error) {
    console.error("Failed to start Zeta", error);
    await application.disposeAfterStartupFailure();
    app.exit(1);
  }
}

function configureProductDataPaths(product: ProductConfiguration): void {
  if (process.platform === "win32") {
    app.setAppUserModelId(product.applicationId);
  }
  const paths = resolveProductDataPaths(app.getPath("appData"), product);
  if (!hasUserDataDirectoryOverride(process.argv)) {
    app.setPath("userData", paths.userDataPath);
  }
  const userDataPath = app.getPath("userData");
  app.setPath("sessionData", join(userDataPath, "session-data"));
  app.setPath("logs", join(userDataPath, "logs"));
  app.setPath("crashDumps", join(userDataPath, "crashes"));
}

function hasUserDataDirectoryOverride(args: readonly string[]): boolean {
  return args.some((argument) =>
    argument === "--user-data-dir" || argument.startsWith("--user-data-dir="),
  );
}
