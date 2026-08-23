import { ZetaDesktopApplication } from "../../product/common/product.js";
import { WorkbenchModeId } from "../../product/common/workbenchMode.js";
import { startElectronApplication } from "./startElectronApplication.js";
import { debugAdapterIpcRoutes } from "../../platform/debug/electron-main/debugAdapterIpcRoutes.js";

/** Compatibility entry that starts the shared application in Code mode. */
startElectronApplication({ application: ZetaDesktopApplication, initialModeId: WorkbenchModeId.Code, ipcRouteContributions: [debugAdapterIpcRoutes] });
