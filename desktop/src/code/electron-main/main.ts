import { app, BrowserWindow, ipcMain } from "electron/main";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  APP_SERVER_SCHEMA_HASH,
} from "../../../generated/app-server/types.js";
import { appServerIpcRoutes } from "../../platform/app-server/electron-main/app-server-ipc.js";
import { AppServerSupervisor } from "../../platform/app-server/electron-main/app-server-supervisor.js";
import {
  normalizeEntryUrl,
  registerTrustedIpcRoutes,
} from "../../platform/app-server/electron-main/trusted-ipc-router.js";

let supervisor: AppServerSupervisor | undefined;

app.whenReady().then(async () => {
  const executableName = process.platform === "win32" ? "zeta.exe" : "zeta";
  const executable = app.isPackaged
    ? join(process.resourcesPath, "bin", executableName)
    : join(app.getAppPath(), "..", "zeta-rs", "target", "debug", executableName);
  supervisor = new AppServerSupervisor({
    executable,
    args: ["app-server", "--listen", "stdio://"],
    environment: {
      PATH: process.env.PATH ?? "",
      ZETA_STATE_ROOT: join(app.getPath("userData"), "state"),
    },
    session: {
      clientName: "zeta-desktop",
      clientVersion: app.getVersion(),
      schemaHash: APP_SERVER_SCHEMA_HASH,
      initializeTimeoutMs: 10_000,
      expectedServerName: "zeta-app-server",
    },
  });
  await supervisor.start();

  const useWindowsTitleBarOverlay = process.platform === "win32";
  const window = new BrowserWindow({
    frame: !useWindowsTitleBarOverlay,
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : undefined,
    titleBarOverlay: useWindowsTitleBarOverlay ? { color: "#181818", symbolColor: "#d6d6d6", height: 35 } : false,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(app.getAppPath(), "dist/preload/src/code/electron-browser/preload.cjs"),
    },
  });
  const rendererUrl = process.env.ZETA_RENDERER_URL;
  const rendererFile = join(
    app.getAppPath(),
    "dist/renderer/electron-browser/workbench/workbench.html",
  );
  const rendererEntryUrl =
    !app.isPackaged && rendererUrl
      ? new URL("/electron-browser/workbench/workbench.html", rendererUrl).href
      : pathToFileURL(rendererFile).href;
  const disposeRoutes = registerTrustedIpcRoutes(
    ipcMain,
    {
      webContents: window.webContents,
      allowedEntryUrls: new Set([normalizeEntryUrl(rendererEntryUrl)]),
    },
    appServerIpcRoutes(supervisor),
  );
  const disposeNotifications = supervisor.onNotification((notification) =>
    window.webContents.send("zeta:event", notification),
  );
  const disposeState = supervisor.onStateChange((state) =>
    window.webContents.send("zeta:app-server:stateChanged", state),
  );
  window.once("closed", () => {
    disposeRoutes();
    disposeNotifications();
    disposeState();
  });

  if (!app.isPackaged && rendererUrl) {
    await window.loadURL(rendererEntryUrl);
  } else {
    await window.loadFile(rendererFile);
  }
});

app.on("before-quit", () => {
  void supervisor?.stop();
});
