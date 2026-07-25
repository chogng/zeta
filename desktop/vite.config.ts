import { resolve } from "node:path";
import { defineConfig } from "vite";

const codeRoot = resolve(import.meta.dirname, "src/code");

export default defineConfig({
  base: "./",
  root: codeRoot,
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: resolve(import.meta.dirname, "dist/renderer"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "browser/workbench/workbench": resolve(codeRoot, "browser/workbench/workbench.html"),
        "electron-browser/workbench/workbench": resolve(codeRoot, "electron-browser/workbench/workbench.html"),
      },
    },
  },
});
