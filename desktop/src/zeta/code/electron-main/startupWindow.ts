import {
  BrowserWindow,
} from "electron/main";
import {
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import {
  createStartupWindowUrl,
} from "./startupWindowPage.js";

export interface StartupWindowOptions {
  readonly productName: string;
  readonly onClosed: () => void;
}

/**
 * Owns the inert window shown before the App Server startup gate completes.
 */
export class StartupWindow extends DisposableOwner {
  readonly #productName: string;
  #window: BrowserWindow | undefined;
  #closedByOwner = false;

  constructor(options: StartupWindowOptions) {
    super();
    this.#productName = options.productName;

    const window = new BrowserWindow({
      width: 520,
      height: 320,
      minWidth: 420,
      minHeight: 260,
      show: false,
      resizable: false,
      maximizable: false,
      fullscreenable: false,
      autoHideMenuBar: true,
      backgroundColor: "#181818",
      title: options.productName,
      webPreferences: {
        contextIsolation: true,
        nodeIntegration: false,
        sandbox: true,
      },
    });
    this.#window = window;
    window.once("closed", () => {
      this.#window = undefined;
      if (!this.#closedByOwner) {
        options.onClosed();
      }
    });
    this.defer(() => {
      this.#closedByOwner = true;
      const ownedWindow = this.#window;
      this.#window = undefined;
      if (ownedWindow && !ownedWindow.isDestroyed()) {
        ownedWindow.close();
      }
    });
  }

  get window(): BrowserWindow | undefined {
    return this.#window;
  }

  get closed(): boolean {
    return !this.#window || this.#window.isDestroyed();
  }

  async showStarting(): Promise<void> {
    await this.#show(
      "starting",
      "Validating the local App Server before opening the Workbench…",
    );
  }

  async showRetrying(): Promise<void> {
    await this.#show(
      "starting",
      "Retrying the secure connection to the App Server…",
    );
  }

  async showFailure(message: string): Promise<void> {
    await this.#show("failed", message);
  }

  complete(): void {
    this.dispose();
  }

  async #show(kind: "starting" | "failed", message: string): Promise<void> {
    const window = this.#window;
    if (!window || window.isDestroyed()) {
      throw new Error("Startup window was closed");
    }
    window.setProgressBar(kind === "starting" ? 2 : -1);
    await window.loadURL(createStartupWindowUrl(
      this.#productName,
      { kind, message },
    ));
    if (!window.isDestroyed()) {
      window.show();
      window.focus();
    }
  }
}
