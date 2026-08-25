import { resolve } from "node:path";
import { defineConfig } from "vite";
import { ZetaRendererDirectory } from "../../zeta-ts/src/zeta/code/common/application.js";
import { WorkbenchModeRegistry } from "../../zeta-ts/src/zeta/workbench/common/workbenchMode.js";
import { desktopBuildPath } from "../lib/paths.ts";
import { hotReloadPlugin } from "./hotReloadPlugin.ts";
import { productIconsPlugin } from "./productIconsPlugin.ts";
import { webAppServerVitePlugin } from "./webAppServerPlugin.ts";
import { workbenchEntryPlugin } from "./workbenchEntryPlugin.ts";

export default defineConfig(() => {
  const desktopRoot = resolve(import.meta.dirname, "../../zeta-ts");
  const repositoryRoot = resolve(desktopRoot, "..");
  const workbenchModeId = WorkbenchModeRegistry.resolveModeId(process.env.ZETA_WORKBENCH_MODE);
  const webAppServerEnabled = process.env.ZETA_WEB_APP_SERVER === "1";
  const developmentPort = webAppServerEnabled ? 5174 : 5173;
  const sourceRoot = resolve(desktopRoot, "src/zeta/code");
  const browserEntry = "browser/workbench/workbench";
  const electronEntry = "electron-browser/workbench/workbench";
  const dedicatedSessionsEntries = WorkbenchModeRegistry.definitions.flatMap(mode => mode.dedicatedSessions ? [mode.dedicatedSessions.rendererEntry] : []);
  const sessionsInputs = Object.fromEntries(dedicatedSessionsEntries.flatMap(rendererEntry => [
    [`browser/sessions/${rendererEntry}`, resolve(sourceRoot, `browser/sessions/${rendererEntry}.html`)],
    [`electron-browser/sessions/${rendererEntry}`, resolve(sourceRoot, `electron-browser/sessions/${rendererEntry}.html`)],
  ]));
  const remoteRuntimeInstallInput = {
    "electron-browser/remote-runtime-install/remoteRuntimeInstall": resolve(sourceRoot, "electron-browser/remote-runtime-install/remoteRuntimeInstall.html"),
  };

  return {
    base: "./",
    root: sourceRoot,
    define: {
      __ZETA_WORKBENCH_MODE__: JSON.stringify(workbenchModeId),
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
      outDir: desktopBuildPath(repositoryRoot, "renderer", ZetaRendererDirectory),
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
