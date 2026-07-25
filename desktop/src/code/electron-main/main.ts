import { app, BrowserWindow, ipcMain } from "electron";
import { join } from "node:path";
import { AppServerProcess } from "../../platform/app-server/electron-main/app-server-process.js";

const server = new AppServerProcess();

function requireTrustedSender(event: Electron.IpcMainInvokeEvent): void {
  if (!event.senderFrame?.url.startsWith("file:")) throw new Error("Untrusted renderer IPC sender");
}

app.whenReady().then(async () => {
  const connection = server.start();
  await connection.request("initialize", {
    clientInfo: { name: "zeta-desktop", version: app.getVersion() },
    protocolVersions: { min: 1, max: 1 },
    capabilities: { notifications: true },
  });
  const window = new BrowserWindow({ webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true, preload: join(import.meta.dirname, "../electron-browser/preload.js") } });
  connection.onNotification((notification) => window.webContents.send("zeta:event", notification));
  ipcMain.handle("zeta:thread:start", (event, params) => { requireTrustedSender(event); return connection.request("thread/start", params); });
  ipcMain.handle("zeta:thread:read", (event, params) => { requireTrustedSender(event); return connection.request("thread/read", params); });
  ipcMain.handle("zeta:turn:start", (event, params) => { requireTrustedSender(event); return connection.request("turn/start", params); });
  ipcMain.handle("zeta:turn:interrupt", (event, params) => { requireTrustedSender(event); return connection.request("turn/interrupt", params); });
  await window.loadFile(join(import.meta.dirname, "../../../../src/code/electron-browser/workbench/workbench.html"));
});

app.on("before-quit", () => server.stop());
