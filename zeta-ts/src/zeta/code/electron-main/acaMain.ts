import { ZetaDesktopApplication } from "../../product/common/product.js";
import { WorkbenchModeId } from "../../product/common/workbenchMode.js";
import { debugAdapterIpcRoutes } from "../../platform/debug/electron-main/debugAdapterIpcRoutes.js";
import { startElectronApplication } from "./startElectronApplication.js";

/** Compatibility entry that starts the shared application in Academic mode. */
startElectronApplication({ application: ZetaDesktopApplication, initialModeId: WorkbenchModeId.Academic, ipcRouteContributions: [debugAdapterIpcRoutes] });
