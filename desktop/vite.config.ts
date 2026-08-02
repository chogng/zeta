import { resolve } from "node:path";
import { defineConfig } from "vite";
import { productIconsPlugin } from "./scripts/product-icons-vite-plugin.mjs";
import { webAppServerVitePlugin } from "./scripts/web-app-server-vite-plugin.mjs";
import { workbenchEntryPlugin } from "./scripts/workbench-entry-vite-plugin.mjs";
import { getProductConfiguration, resolveProductId } from "./src/zeta/product/common/product.js";

export default defineConfig(() => {
  const product = getProductConfiguration(resolveProductId(process.env.ZETA_PRODUCT));
  const webAppServerEnabled = process.env.ZETA_WEB_APP_SERVER === "1";
  const developmentPort = webAppServerEnabled ? 5174 : 5173;
  const sourceRoot = resolve(import.meta.dirname, "src/zeta/code");
  const browserEntry = `browser/workbench/${product.rendererEntry}`;
  const electronEntry = `electron-browser/workbench/${product.rendererEntry}`;

  return {
    base: "./",
    root: sourceRoot,
    define: {
      __ZETA_WEB_APP_SERVER__: JSON.stringify(webAppServerEnabled),
    },
    plugins: [workbenchEntryPlugin(product.rendererEntry), productIconsPlugin(), ...(webAppServerEnabled ? [webAppServerVitePlugin()] : [])],
    server: {
      host: "127.0.0.1",
      port: developmentPort,
      strictPort: true,
    },
    build: {
      outDir: resolve(import.meta.dirname, `dist/renderer/${product.id}`),
      emptyOutDir: true,
      rollupOptions: {
        input: {
          [browserEntry]: resolve(sourceRoot, `${browserEntry}.html`),
          [electronEntry]: resolve(sourceRoot, `${electronEntry}.html`),
        },
      },
    },
  };
});
