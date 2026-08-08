import { bootstrapElectronMain } from "./bootstrap.js";

bootstrapElectronMain();
await import(productMainEntry());

function productMainEntry(): string {
  const entry = process.env.ZETA_ELECTRON_MAIN;
  if (entry === undefined || entry.length === 0) {
    return "./zeta/code/electron-main/main.js";
  }
  if (entry === "code" || entry === "academic") {
    const configuredProduct = process.env.ZETA_PRODUCT;
    if (configuredProduct !== undefined && configuredProduct !== entry) {
      throw new Error(`Electron Main entry '${entry}' conflicts with ZETA_PRODUCT '${configuredProduct}'`);
    }
    return entry === "code"
      ? "./zeta/code/electron-main/codeMain.js"
      : "./zeta/code/electron-main/acaMain.js";
  }
  throw new TypeError(`Unknown Electron Main entry '${entry}'. Expected code or academic`);
}
