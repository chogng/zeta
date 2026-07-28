import { app } from "electron/main";
import { join } from "node:path";
import {
  getProductConfiguration,
  resolveProductId,
} from "../../product/common/product.js";
import {
  resolvePackagedProductId,
} from "../../product/node/product.js";
import {
  ZetaApplication,
} from "./app.js";

const rendererRoot = join(app.getAppPath(), "dist", "renderer");
const product = getProductConfiguration(
  app.isPackaged
    ? resolvePackagedProductId(rendererRoot)
    : resolveProductId(process.env.ZETA_PRODUCT),
);

app.setName(product.name);

const application = ZetaApplication.create({
  product,
  rendererRoot,
});

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
