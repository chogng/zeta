import { app, BrowserWindow, ipcMain, Menu, screen } from "electron/main";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  APP_SERVER_SCHEMA_HASH,
} from "../../../generated/app-server/types.js";
import {
  DisposableStore,
  type IDisposable,
} from "../../base/common/lifecycle.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../base/common/disposableTracker.js";
import {
  ElectronContextMenu,
} from "../../base/parts/contextmenu/electron-main/contextmenu.js";
import { appServerIpcRoutes } from "../../platform/app-server/electron-main/app-server-ipc.js";
import { AppServerSupervisor } from "../../platform/app-server/electron-main/app-server-supervisor.js";
import {
  normalizeEntryUrl,
  registerTrustedIpcRoutes,
} from "../../platform/app-server/electron-main/trusted-ipc-router.js";
import {
  nativeContextMenuIpcRoutes,
} from "../../platform/contextview/electron-main/contextMenuIpc.js";
import {
  CONFIGURATION_CHANGED_CHANNEL,
} from "../../platform/configuration/common/configuration.js";
import {
  ConfigurationMainService,
  configurationIpcRoutes,
} from "../../platform/configuration/electron-main/configurationMainService.js";
import {
  KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  migrateLegacyKeybindings,
} from "../../platform/keybinding/electron-main/migrateLegacyKeybindings.js";
import {
  KeybindingsResourceMainService,
  keybindingsResourceIpcRoutes,
} from "../../platform/keybinding/electron-main/keybindingsResourceMainService.js";
import {
  NativeMenubarMainService,
  nativeMenubarIpcRoutes,
} from "../../platform/menubar/electron-main/menubarMainService.js";
import { StateService } from "../../platform/state/node/stateService.js";
import {
  WindowMode,
} from "../../platform/window/electron-main/window.js";
import {
  applyWindowState,
  resolveBrowserWindowOptions,
} from "../../platform/windows/electron-main/windows.js";
import {
  WindowsStateHandler,
} from "../../platform/windows/electron-main/windowsStateHandler.js";
import {
  type IAnyWorkspaceIdentifier,
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
  workbenchStateFromWorkspaceIdentifier,
} from "../../platform/workspace/common/workspace.js";
import {
  WorkspacesMainService,
  workspaceContextIpcRoutes,
} from "../../platform/workspaces/electron-main/workspacesMainService.js";

let supervisor: AppServerSupervisor | undefined;
let mainWindow: BrowserWindow | undefined;
let stateService: StateService | undefined;
let configurationService: ConfigurationMainService | undefined;
let keybindingsResourceService: KeybindingsResourceMainService | undefined;
let windowsStateHandler: WindowsStateHandler | undefined;
let windowStateTracking: IDisposable | undefined;
let quitAfterStateSaved = false;
let quitSaveStarted = false;
const disposableTracker = app.isPackaged
  ? undefined
  : new DisposableTracker();
const tracking = disposableTracker
  ? installDisposableTracker(disposableTracker)
  : undefined;

app.whenReady().then(async () => {
  if (process.platform !== "darwin") {
    Menu.setApplicationMenu(null);
  }
  stateService = await StateService.create(
    join(app.getPath("userData"), "state.json"),
  );
  configurationService = await ConfigurationMainService.create({
    filePath: join(app.getPath("userData"), "configuration.json"),
    onError: (error) => {
      console.error("Failed to process configuration", error);
    },
  });
  keybindingsResourceService = await KeybindingsResourceMainService.create({
    filePath: join(app.getPath("userData"), "keybindings.json"),
    onError: (error) => {
      console.error("Failed to process keybindings resource", error);
    },
  });
  await migrateLegacyKeybindings(
    configurationService,
    keybindingsResourceService,
  );
  const workspacesMainService = new WorkspacesMainService();
  let workspace: IAnyWorkspaceIdentifier;
  try {
    workspace = await workspacesMainService.resolveStartupWorkspace({
      arguments: process.argv.slice(app.isPackaged ? 1 : 2),
      cwd: process.cwd(),
    });
  } catch (error) {
    console.error("Failed to resolve startup workspace", error);
    workspace = UNKNOWN_EMPTY_WINDOW_WORKSPACE;
  }
  windowsStateHandler = new WindowsStateHandler({
    stateService,
    workbenchState: workbenchStateFromWorkspaceIdentifier(workspace),
    displayService: {
      getAllDisplays: () => screen.getAllDisplays(),
      getDisplayMatching: (bounds) => screen.getDisplayMatching(bounds),
    },
    onError: (error) => {
      console.error("Failed to save window state", error);
    },
  });
  const windowState = windowsStateHandler.restoreWindowState();

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

  const browserWindowOptions = resolveBrowserWindowOptions({
    state: windowState,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(app.getAppPath(), "dist/preload/src/code/electron-browser/preload.cjs"),
      additionalArguments: [],
    },
  });
  const window = new BrowserWindow(browserWindowOptions);
  mainWindow = window;
  windowStateTracking = windowsStateHandler.trackWindow(window);
  if (windowState.mode !== WindowMode.Normal) {
    window.once("ready-to-show", () => {
      if (!window.isDestroyed()) {
        window.show();
      }
    });
    applyWindowState(window, windowState);
  }
  const rendererUrl = process.env.ZETA_RENDERER_URL;
  const rendererFile = join(
    app.getAppPath(),
    "dist/renderer/electron-browser/workbench/workbench.html",
  );
  const rendererEntryUrl =
    !app.isPackaged && rendererUrl
      ? new URL("/electron-browser/workbench/workbench.html", rendererUrl).href
      : pathToFileURL(rendererFile).href;
  const windowDisposables = new DisposableStore();
  windowDisposables.add(windowStateTracking);
  const ipcRoutes = [
    ...appServerIpcRoutes(supervisor),
    ...configurationIpcRoutes(configurationService),
    ...keybindingsResourceIpcRoutes(keybindingsResourceService),
    ...workspaceContextIpcRoutes(workspace),
  ];
  if (process.platform === "darwin") {
    const nativeContextMenu = windowDisposables.add(
      new ElectronContextMenu(window),
    );
    const nativeMenubar = windowDisposables.add(
      new NativeMenubarMainService(window),
    );
    ipcRoutes.push(...nativeContextMenuIpcRoutes(nativeContextMenu));
    ipcRoutes.push(...nativeMenubarIpcRoutes(nativeMenubar));
  }
  windowDisposables.add(registerTrustedIpcRoutes(
    ipcMain,
    {
      webContents: window.webContents,
      allowedEntryUrls: new Set([normalizeEntryUrl(rendererEntryUrl)]),
    },
    ipcRoutes,
  ));
  windowDisposables.add(supervisor.onNotification((notification) =>
    window.webContents.send("zeta:event", notification),
  ));
  windowDisposables.add(supervisor.onStateChange((state) =>
    window.webContents.send("zeta:app-server:stateChanged", state),
  ));
  windowDisposables.add(configurationService.onDidChange((snapshot) =>
    window.webContents.send(
      CONFIGURATION_CHANGED_CHANNEL,
      snapshot,
    ),
  ));
  windowDisposables.add(keybindingsResourceService.onDidChange((snapshot) =>
    window.webContents.send(
      KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
      snapshot,
    ),
  ));
  window.once("closed", () => {
    windowDisposables.dispose();
    if (mainWindow === window) {
      mainWindow = undefined;
      windowStateTracking = undefined;
    }
  });

  if (!app.isPackaged && rendererUrl) {
    await window.loadURL(rendererEntryUrl);
  } else {
    await window.loadFile(rendererFile);
  }
});

app.on("before-quit", (event) => {
  supervisor?.dispose();

  if (quitAfterStateSaved || !stateService) {
    return;
  }
  event.preventDefault();
  if (quitSaveStarted) {
    return;
  }

  quitSaveStarted = true;
  windowStateTracking?.dispose();
  void (async () => {
    try {
      if (mainWindow && !mainWindow.isDestroyed()) {
        await windowsStateHandler?.saveWindowState(mainWindow);
      }
      await Promise.all([
        stateService?.close(),
        configurationService?.close(),
        keybindingsResourceService?.close(),
      ]);
    } catch (error) {
      console.error("Failed to flush application state before quit", error);
    } finally {
      quitAfterStateSaved = true;
      app.quit();
    }
  })();
});

app.on("will-quit", () => {
  try {
    disposableTracker?.assertNoLeaks();
  } finally {
    tracking?.[Symbol.dispose]();
  }
});
