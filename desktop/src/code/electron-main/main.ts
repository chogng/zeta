import { app, BrowserWindow, ipcMain, type IpcMainInvokeEvent } from "electron/main";
import { join } from "node:path";
import { AppServerProcess } from "../../platform/app-server/electron-main/app-server-process.js";

const server = new AppServerProcess();

function requireTrustedSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame?.url;
  if (senderUrl?.startsWith("file:")) return;
  const rendererUrl = process.env.ZETA_RENDERER_URL;
  if (!app.isPackaged && senderUrl && rendererUrl && new URL(senderUrl).origin === new URL(rendererUrl).origin) return;
  throw new Error("Untrusted renderer IPC sender");
}

app.whenReady().then(async () => {
  const connection = server.start();
  await connection.request("initialize", {
    clientInfo: { name: "zeta-desktop", version: app.getVersion() },
    protocolVersions: { min: 1, max: 1 },
    capabilities: { notifications: true },
  });
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
  connection.onNotification((notification) => window.webContents.send("zeta:event", notification));
  ipcMain.handle("zeta:thread:start", (event, params) => { requireTrustedSender(event); return connection.request("thread/start", params); });
  ipcMain.handle("zeta:thread:read", (event, params) => { requireTrustedSender(event); return connection.request("thread/read", params); });
  ipcMain.handle("zeta:turn:start", (event, params) => { requireTrustedSender(event); return connection.request("turn/start", params); });
  ipcMain.handle("zeta:turn:interrupt", (event, params) => { requireTrustedSender(event); return connection.request("turn/interrupt", params); });
  const rendererUrl = process.env.ZETA_RENDERER_URL;
  if (!app.isPackaged && rendererUrl) {
    await window.loadURL(new URL("/electron-browser/workbench/workbench.html", rendererUrl).href);
  } else {
    await window.loadFile(join(app.getAppPath(), "dist/renderer/electron-browser/workbench/workbench.html"));
  }
});

app.on("before-quit", () => server.stop());
