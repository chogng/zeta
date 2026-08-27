import { resolve } from "node:path";
import { defineConfig } from "vite";
import { hotReloadPlugin } from "./hotReloadPlugin.ts";

const repositoryRoot = resolve(import.meta.dirname, "../..");
const desktopRoot = resolve(repositoryRoot, "zeta-ts");

export default defineConfig({
  base: "./",
  root: repositoryRoot,
  plugins: [hotReloadPlugin({ desktopRoot })],
  server: {
    host: "127.0.0.1",
    port: 5199,
    strictPort: true,
  },
  build: {
    outDir: resolve(repositoryRoot, ".build/desktop/stanza"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        stanza: resolve(import.meta.dirname, "stanza/index.html"),
      },
    },
  },
});
