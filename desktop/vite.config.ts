import { resolve } from "node:path";
import { defineConfig } from "vite";
import { productIconsPlugin } from "./scripts/product-icons-vite-plugin.mjs";
import {
  getProductConfiguration,
  resolveProductId,
} from "./src/zeta/product/common/product.js";

await import("./scripts/sync-generated-resources.mjs");

export default defineConfig(() => {
  const product = getProductConfiguration(
    resolveProductId(process.env.ZETA_PRODUCT),
  );
  const sourceRoot = resolve(import.meta.dirname, "src/zeta/code");
  const browserEntry = `browser/workbench/${product.rendererEntry}`;
  const electronEntry =
    `electron-browser/workbench/${product.rendererEntry}`;

  return {
    base: "./",
    root: sourceRoot,
    plugins: [productIconsPlugin()],
    server: {
      host: "127.0.0.1",
      port: 5173,
      strictPort: true,
    },
    build: {
      outDir: resolve(
        import.meta.dirname,
        `dist/renderer/${product.id}`,
      ),
      emptyOutDir: true,
      rollupOptions: {
        input: {
          [browserEntry]: resolve(
            sourceRoot,
            `${browserEntry}.html`,
          ),
          [electronEntry]: resolve(
            sourceRoot,
            `${electronEntry}.html`,
          ),
        },
      },
    },
  };
});
