import { existsSync } from "node:fs";
import { join } from "node:path";
import { getProductConfiguration, productIds, } from "../common/product.js";
/**
 * Resolves the product identity baked into a packaged renderer tree.
 *
 * Release packaging must include exactly one product directory. Multiple
 * directories are rejected so installed application identity never depends on
 * a user-controlled environment variable.
 */
export function resolvePackagedProductId(rendererRoot) {
    const packagedProducts = productIds.filter((productId) => existsSync(join(rendererRoot, productId, "electron-browser", "workbench", `${getProductConfiguration(productId).rendererEntry}.html`)));
    if (packagedProducts.length !== 1) {
        throw new Error("Packaged Zeta must contain exactly one renderer product; " +
            `found ${packagedProducts.join(", ") || "none"}`);
    }
    return packagedProducts[0];
}
