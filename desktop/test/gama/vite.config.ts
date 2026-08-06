import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  root: resolve(import.meta.dirname),
  server: { host: "127.0.0.1", port: 5186, strictPort: true },
  build: {
    outDir: resolve(import.meta.dirname, "dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        gama: resolve(import.meta.dirname, "gama.html"),
      },
    },
  },
});
