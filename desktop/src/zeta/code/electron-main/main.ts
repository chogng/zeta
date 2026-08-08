import { app } from "electron/main";
import { join } from "node:path";
import {
  getProductConfiguration,
  resolveProductId,
} from "../../product/common/product.js";
import { resolvePackagedProductId, resolveProductDataPaths } from "../../product/node/product.js";
import {
  type AppServerStartupMode,
  ZetaApplication,
} from "./app.js";

const rendererRoot = join(app.getAppPath(), "dist", "renderer");
const appServerStartupMode: AppServerStartupMode = process.env.ZETA_DESKTOP_UI_ONLY === "1"
  ? "disabled"
  : "required";
const product = getProductConfiguration(
  app.isPackaged
    ? resolvePackagedProductId(rendererRoot)
    : resolveProductId(process.env.ZETA_PRODUCT),
);

app.setName(product.name);
configureProductDataPaths(product);

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  const application = ZetaApplication.create({
    product,
    rendererRoot,
    appServerStartupMode,
  });

  app.on("second-instance", () => application.focusMainWindow());

  async function startup(): Promise<void> {
    try {
      await application.startupAfterReady();
    } catch (error) {
      console.error("Failed to start Zeta", error);
      await application.disposeAfterStartupFailure();
      app.exit(1);
    }
  }

  app.once("ready", () => {
    void startup();
  });
}

function configureProductDataPaths(
  product: ReturnType<typeof getProductConfiguration>,
): void {
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
