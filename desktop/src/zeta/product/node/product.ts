import { existsSync } from "node:fs";
import { join } from "node:path";
import { getProductConfiguration, type ProductConfiguration, type ProductId, productIds } from "../common/product.js";

export interface ProductDataPaths {
  readonly userDataPath: string;
  readonly sessionDataPath: string;
}

/** Resolves the persistent Electron roots for one product edition. */
export function resolveProductDataPaths(appDataPath: string, product: ProductConfiguration): ProductDataPaths {
  if (appDataPath.trim().length === 0) throw new TypeError("Application data path must not be empty");
  if (product.userDataFolderName.trim().length === 0) throw new TypeError("Product user data folder name must not be empty");
  const userDataPath = join(appDataPath, product.userDataFolderName);
  return { userDataPath, sessionDataPath: join(userDataPath, "session-data") };
}

/**
 * Resolves the product identity baked into a packaged renderer tree.
 *
 * Release packaging must include exactly one product directory. Multiple
 * directories are rejected so installed application identity never depends on
 * a user-controlled environment variable.
 */
export function resolvePackagedProductId(
  rendererRoot: string,
): ProductId {
  const packagedProducts = productIds.filter((productId) =>
    existsSync(join(
      rendererRoot,
      productId,
      "electron-browser",
      "workbench",
      `${getProductConfiguration(productId).rendererEntry}.html`,
    ))
  );
  if (packagedProducts.length !== 1) {
    throw new Error(
      "Packaged Zeta must contain exactly one renderer product; " +
        `found ${packagedProducts.join(", ") || "none"}`,
    );
  }
  return packagedProducts[0];
}
