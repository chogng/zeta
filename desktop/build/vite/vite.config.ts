import { resolve } from "node:path";
import { defineConfig } from "vite";
import { getProductConfiguration, resolveProductId } from "../../src/zeta/product/common/product.js";
import { hotReloadPlugin } from "./hotReloadPlugin.ts";
import { productIconsPlugin } from "./productIconsPlugin.mjs";
import { webAppServerVitePlugin } from "./webAppServerPlugin.mjs";
import { workbenchEntryPlugin } from "./workbenchEntryPlugin.mjs";

export default defineConfig(() => {
  const desktopRoot = resolve(import.meta.dirname, "../..");
  const product = getProductConfiguration(resolveProductId(process.env.ZETA_PRODUCT));
  const webAppServerEnabled = process.env.ZETA_WEB_APP_SERVER === "1";
  const developmentPort = webAppServerEnabled ? 5174 : 5173;
  const sourceRoot = resolve(desktopRoot, "src/zeta/code");
  const browserEntry = "browser/workbench/workbench";
  const electronEntry = "electron-browser/workbench/workbench";
  const dedicatedSessions = product.dedicatedSessions;
  const sessionsInputs = dedicatedSessions
    ? {
        [`browser/sessions/${dedicatedSessions.rendererEntry}`]: resolve(sourceRoot, `browser/sessions/${dedicatedSessions.rendererEntry}.html`),
        [`electron-browser/sessions/${dedicatedSessions.rendererEntry}`]: resolve(sourceRoot, `electron-browser/sessions/${dedicatedSessions.rendererEntry}.html`),
      }
    : {};
  const remoteRuntimeInstallInput = {
    "electron-browser/remote-runtime-install/remoteRuntimeInstall": resolve(sourceRoot, "electron-browser/remote-runtime-install/remoteRuntimeInstall.html"),
  };

  return {
    base: "./",
    root: sourceRoot,
    define: {
      __ZETA_PRODUCT__: JSON.stringify(product.id),
      __ZETA_WEB_APP_SERVER__: JSON.stringify(webAppServerEnabled),
    },
    plugins: [hotReloadPlugin({ desktopRoot }), workbenchEntryPlugin(), productIconsPlugin(), ...(webAppServerEnabled ? [webAppServerVitePlugin()] : [])],
    optimizeDeps: {
      include: ["vscode-oniguruma"],
    },
    server: {
      host: "127.0.0.1",
      port: developmentPort,
      strictPort: true,
    },
    build: {
      outDir: resolve(desktopRoot, `dist/renderer/${product.id}`),
      emptyOutDir: true,
      rollupOptions: {
        input: {
          [browserEntry]: resolve(sourceRoot, `${browserEntry}.html`),
          [electronEntry]: resolve(sourceRoot, `${electronEntry}.html`),
          ...sessionsInputs,
          ...remoteRuntimeInstallInput,
        },
      },
    },
  };
});
