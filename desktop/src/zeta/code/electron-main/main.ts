import { app } from "electron/main";
import { join } from "node:path";
import { getProductConfiguration, resolveProductId } from "../../product/common/product.js";
import { resolvePackagedProductId } from "../../product/node/product.js";
import { startElectronApplication } from "./startElectronApplication.js";

const rendererRoot = join(app.getAppPath(), "dist", "renderer");
const product = getProductConfiguration(
  app.isPackaged
    ? resolvePackagedProductId(rendererRoot)
    : resolveProductId(process.env.ZETA_PRODUCT),
);

startElectronApplication(product);
