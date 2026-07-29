import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  screen,
  type Event as ElectronEvent,
} from "electron/main";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  APP_SERVER_SCHEMA_HASH,
} from "../../../../generated/app-server/types.js";
import {
  DisposableOwner,
  DisposableStore,
  type IDisposable,
} from "../../base/common/lifecycle.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../base/common/disposableTracker.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import {
  ElectronContextMenu,
} from "../../base/parts/contextmenu/electron-main/contextmenu.js";
import {
  appServerIpcRoutes,
} from "../../platform/app-server/electron-main/app-server-ipc.js";
import {
  AppServerSupervisor,
} from "../../platform/app-server/electron-main/app-server-supervisor.js";
import {
  normalizeEntryUrl,
  registerTrustedIpcRoutes,
} from "../../platform/app-server/electron-main/trusted-ipc-router.js";
import {
  BROWSER_VIEW_EVENT_CHANNEL,
} from "../../platform/browser/common/browserView.js";
import {
  browserViewIpcRoutes,
} from "../../platform/browser/electron-main/browserViewIpc.js";
import {
  BrowserViewMainService,
} from "../../platform/browser/electron-main/browserViewMainService.js";
import {
  CONFIGURATION_CHANGED_CHANNEL,
} from "../../platform/configuration/common/configuration.js";
import {
  ConfigurationMainService,
  configurationIpcRoutes,
} from "../../platform/configuration/electron-main/configurationMainService.js";
import {
  nativeContextMenuIpcRoutes,
} from "../../platform/contextview/electron-main/contextMenuIpc.js";
import {
  KEYBINDINGS_RESOURCE_CHANGED_CHANNEL,
} from "../../platform/keybinding/common/keybindingsResource.js";
import {
  KeybindingsResourceMainService,
  keybindingsResourceIpcRoutes,
} from "../../platform/keybinding/electron-main/keybindingsResourceMainService.js";
import {
  migrateLegacyKeybindings,
} from "../../platform/keybinding/electron-main/migrateLegacyKeybindings.js";
import {
  NativeMenubarMainService,
  nativeMenubarIpcRoutes,
} from "../../platform/menubar/electron-main/menubarMainService.js";
import {
  nativeHostIpcRoutes,
} from "../../platform/native/electron-main/nativeHostIpc.js";
import {
  StateService,
} from "../../platform/state/node/stateService.js";
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
  isSingleFolderWorkspaceIdentifier,
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
} from "../../platform/workspace/common/workspace.js";
import {
  folderRelaunchArguments,
  WorkspacesMainService,
  workspaceContextIpcRoutes,
} from "../../platform/workspaces/electron-main/workspacesMainService.js";
import {
  StartupWindow,
} from "./startupWindow.js";

export interface ZetaApplicationOptions {
  readonly product: ProductConfiguration;
  readonly rendererRoot: string;
}

/**
 * Owns the Electron application's services, primary window, IPC, and shutdown.
 */
export class ZetaApplication extends DisposableOwner {
  readonly #product: ProductConfiguration;
  readonly #rendererRoot: string;
  readonly #disposableTracker: DisposableTracker | undefined;
  readonly #tracking: Disposable | undefined;

  #supervisor: AppServerSupervisor | undefined;
  #mainWindow: BrowserWindow | undefined;
  #startupWindow: StartupWindow | undefined;
  #stateService: StateService | undefined;
  #configurationService: ConfigurationMainService | undefined;
  #keybindingsResourceService: KeybindingsResourceMainService | undefined;
  #windowsStateHandler: WindowsStateHandler | undefined;
  #windowStateTracking: IDisposable | undefined;
  #closePersistentServicesPromise: Promise<void> | undefined;
  #quitAfterStateSaved = false;
  #quitSaveStarted = false;

  private constructor(
    options: ZetaApplicationOptions,
    disposableTracker: DisposableTracker | undefined,
    tracking: Disposable | undefined,
  ) {
    super();
    this.#product = options.product;
    this.#rendererRoot = options.rendererRoot;
    this.#disposableTracker = disposableTracker;
    this.#tracking = tracking;

    app.on("before-quit", this.#onBeforeQuit);
    app.on("will-quit", this.#onWillQuit);
    this.defer(() => {
      app.removeListener("before-quit", this.#onBeforeQuit);
      app.removeListener("will-quit", this.#onWillQuit);
    });
  }

  static create(options: ZetaApplicationOptions): ZetaApplication {
    const disposableTracker = app.isPackaged
      ? undefined
      : new DisposableTracker();
    const tracking = disposableTracker
      ? installDisposableTracker(disposableTracker)
      : undefined;
    return new ZetaApplication(options, disposableTracker, tracking);
  }

  async startupAfterReady(): Promise<void> {
    if (!app.isReady()) {
      throw new Error("Zeta application startup requires Electron to be ready");
    }
    if (process.platform !== "darwin") {
      Menu.setApplicationMenu(null);
    }

    const startupWindow = this.own(new StartupWindow({
      productName: this.#product.name,
      onClosed: () => app.quit(),
    }));
    this.#startupWindow = startupWindow;
    await startupWindow.showStarting();

    await this.#createPersistentServices();
    const workspace = await this.#resolveWorkspace();
    this.#windowsStateHandler = new WindowsStateHandler({
      stateService: this.#stateService!,
      workspace,
      displayService: {
        getAllDisplays: () => screen.getAllDisplays(),
        getDisplayMatching: (bounds) => screen.getDisplayMatching(bounds),
      },
      onError: (error) => {
        console.error("Failed to save window state", error);
      },
    });

    const supervisor = this.own(this.#createAppServerSupervisor(workspace));
    this.#supervisor = supervisor;
    const appServerReady = await this.#startAppServerWithRecovery(
      supervisor,
      startupWindow,
    );
    if (!appServerReady) {
      return;
    }
    await this.#openFirstWindow(workspace, supervisor);
    startupWindow.complete();
    this.#startupWindow = undefined;
  }

  async disposeAfterStartupFailure(): Promise<void> {
    this.#supervisor?.dispose();
    try {
      await this.#closePersistentServices();
    } finally {
      this.dispose();
      this.#releaseDisposableTracker();
    }
  }

  async #createPersistentServices(): Promise<void> {
    this.#stateService = await StateService.create(
      join(app.getPath("userData"), "state.json"),
    );
    this.#configurationService = await ConfigurationMainService.create({
      filePath: join(app.getPath("userData"), "configuration.json"),
      onError: (error) => {
        console.error("Failed to process configuration", error);
      },
    });
    this.#keybindingsResourceService =
      await KeybindingsResourceMainService.create({
        filePath: join(app.getPath("userData"), "keybindings.json"),
        onError: (error) => {
          console.error("Failed to process keybindings resource", error);
        },
      });
    await migrateLegacyKeybindings(
      this.#configurationService,
      this.#keybindingsResourceService,
    );
  }

  async #resolveWorkspace(): Promise<IAnyWorkspaceIdentifier> {
    try {
      return await new WorkspacesMainService().resolveStartupWorkspace({
        arguments: process.argv.slice(app.isPackaged ? 1 : 2),
        cwd: process.cwd(),
      });
    } catch (error) {
      console.error("Failed to resolve startup workspace", error);
      return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
    }
  }

  #createAppServerSupervisor(
    workspace: IAnyWorkspaceIdentifier,
  ): AppServerSupervisor {
    const executableName = process.platform === "win32" ? "zeta.exe" : "zeta";
    const executable = app.isPackaged
      ? join(process.resourcesPath, "bin", executableName)
      : join(
        app.getAppPath(),
        "..",
        "zeta-rs",
        "target",
        "debug",
        executableName,
      );
    return new AppServerSupervisor({
      executable,
      args: ["app-server", "--listen", "stdio://"],
      environment: {
        PATH: process.env.PATH ?? "",
        ...(process.env.ZETA_RG_PATH
          ? { ZETA_RG_PATH: process.env.ZETA_RG_PATH }
          : {}),
        ZETA_STATE_ROOT: join(app.getPath("userData"), "state"),
        ...(isSingleFolderWorkspaceIdentifier(workspace)
          ? { ZETA_WORKSPACE_ROOT: workspace.uri.fsPath }
          : {}),
      },
      session: {
        clientName: "zeta-desktop",
        clientVersion: app.getVersion(),
        schemaHash: APP_SERVER_SCHEMA_HASH,
        initializeTimeoutMs: 10_000,
        expectedServerName: "zeta-app-server",
      },
    });
  }

  async #startAppServerWithRecovery(
    supervisor: AppServerSupervisor,
    startupWindow: StartupWindow,
  ): Promise<boolean> {
    let retrying = false;
    while (!startupWindow.closed) {
      if (retrying) {
        await startupWindow.showRetrying();
      }
      try {
        await supervisor.start();
        return true;
      } catch (error) {
        console.error("App Server failed the startup gate", error);
        if (startupWindow.closed) {
          return false;
        }

        const message = error instanceof Error
          ? error.message
          : "The App Server failed to start";
        await startupWindow.showFailure(message);
        const parent = startupWindow.window;
        if (!parent || parent.isDestroyed()) {
          return false;
        }
        const diagnostics = supervisor.diagnostics().trim();
        const detail = diagnostics
          ? `${message}\n\nDiagnostics:\n${diagnostics}`.slice(0, 8_000)
          : message;
        const result = await dialog.showMessageBox(parent, {
          type: "error",
          title: `${this.#product.name} startup failed`,
          message: "The App Server could not be validated.",
          detail,
          buttons: ["Retry", "Quit"],
          defaultId: 0,
          cancelId: 1,
          noLink: true,
        });
        if (result.response !== 0) {
          app.quit();
          return false;
        }
        await supervisor.stop();
        retrying = true;
      }
    }
    return false;
  }

  async #openFirstWindow(
    workspace: IAnyWorkspaceIdentifier,
    supervisor: AppServerSupervisor,
  ): Promise<void> {
    const windowsStateHandler = this.#windowsStateHandler!;
    const windowState = windowsStateHandler.restoreWindowState();
    const browserWindowOptions = resolveBrowserWindowOptions({
      state: windowState,
      webPreferences: {
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
        preload: join(
          app.getAppPath(),
          "dist/preload/src/zeta/base/parts/sandbox/electron-browser/preload.cjs",
        ),
        additionalArguments: [],
      },
    });
    const window = new BrowserWindow(browserWindowOptions);
    this.#mainWindow = window;
    this.#windowStateTracking = windowsStateHandler.trackWindow(window);
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
      this.#rendererRoot,
      this.#product.id,
      "electron-browser",
      "workbench",
      `${this.#product.rendererEntry}.html`,
    );
    const rendererEntryUrl =
      !app.isPackaged && rendererUrl
        ? new URL(
          `/electron-browser/workbench/${this.#product.rendererEntry}.html`,
          rendererUrl,
        ).href
        : pathToFileURL(rendererFile).href;

    const windowDisposables = this.own(new DisposableStore());
    windowDisposables.add(this.#windowStateTracking);
    const browserViewMainService = windowDisposables.add(
      new BrowserViewMainService({
        window,
        emitEvent: (event) => {
          if (!window.isDestroyed()) {
            window.webContents.send(BROWSER_VIEW_EVENT_CHANNEL, event);
          }
        },
      }),
    );
    const configurationService = this.#configurationService!;
    const keybindingsResourceService = this.#keybindingsResourceService!;
    const ipcRoutes = [
      ...appServerIpcRoutes(supervisor),
      ...browserViewIpcRoutes(browserViewMainService),
      ...configurationIpcRoutes(configurationService),
      ...keybindingsResourceIpcRoutes(keybindingsResourceService),
      ...nativeHostIpcRoutes({
        openFolder: async () => {
          const result = await dialog.showOpenDialog(window, {
            title: "Open Folder",
            properties: ["openDirectory"],
          });
          const folderPath = result.filePaths[0];
          if (result.canceled || !folderPath) return;
          app.relaunch({
            args: [...folderRelaunchArguments({
              appPath: app.getAppPath(),
              folderPath,
              isPackaged: app.isPackaged,
            })],
          });
          globalThis.setTimeout(() => app.quit(), 0);
        },
        toggleDeveloperTools: () => window.webContents.toggleDevTools(),
      }),
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
      window.webContents.send("zeta:event", notification)
    ));
    windowDisposables.add(supervisor.onStateChange((state) =>
      window.webContents.send("zeta:app-server:stateChanged", state)
    ));
    windowDisposables.add(configurationService.onDidChange((snapshot) =>
      window.webContents.send(CONFIGURATION_CHANGED_CHANNEL, snapshot)
    ));
    windowDisposables.add(keybindingsResourceService.onDidChange((snapshot) =>
      window.webContents.send(KEYBINDINGS_RESOURCE_CHANGED_CHANNEL, snapshot)
    ));
    window.once("closed", () => {
      windowDisposables.dispose();
      if (this.#mainWindow === window) {
        this.#mainWindow = undefined;
        this.#windowStateTracking = undefined;
      }
    });

    if (!app.isPackaged && rendererUrl) {
      await window.loadURL(rendererEntryUrl);
    } else {
      await window.loadFile(rendererFile);
    }
  }

  readonly #onBeforeQuit = (event: ElectronEvent): void => {
    this.#supervisor?.dispose();
    if (this.#quitAfterStateSaved || !this.#stateService) {
      return;
    }
    event.preventDefault();
    if (this.#quitSaveStarted) {
      return;
    }

    this.#quitSaveStarted = true;
    this.#windowStateTracking?.dispose();
    void (async () => {
      try {
        if (this.#mainWindow && !this.#mainWindow.isDestroyed()) {
          await this.#windowsStateHandler?.saveWindowState(this.#mainWindow);
        }
        await this.#closePersistentServices();
      } catch (error) {
        console.error("Failed to flush application state before quit", error);
      } finally {
        this.#quitAfterStateSaved = true;
        app.quit();
      }
    })();
  };

  readonly #onWillQuit = (): void => {
    this.dispose();
    this.#releaseDisposableTracker();
  };

  #closePersistentServices(): Promise<void> {
    this.#closePersistentServicesPromise ??= Promise.all([
      this.#stateService?.close(),
      this.#configurationService?.close(),
      this.#keybindingsResourceService?.close(),
    ]).then(() => undefined);
    return this.#closePersistentServicesPromise;
  }

  #releaseDisposableTracker(): void {
    try {
      this.#disposableTracker?.assertNoLeaks();
    } finally {
      this.#tracking?.[Symbol.dispose]();
    }
  }
}
