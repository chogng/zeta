import { ZetaDesktopProduct } from "../../product/common/product.js";
import { startElectronApplication } from "./startElectronApplication.js";
import { debugAdapterIpcRoutes } from "../../platform/debug/electron-main/debugAdapterIpcRoutes.js";

/** Code product's explicit Electron Main entry. */
startElectronApplication(ZetaDesktopProduct, [debugAdapterIpcRoutes]);
