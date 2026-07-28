import { toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import type { IStateService } from "../../state/node/state.js";
import {
  WorkbenchState,
} from "../../workspace/common/workspace.js";
import {
  defaultWindowState,
  WindowMode,
  type IWindowBounds,
  type IWindowState,
} from "../../window/electron-main/window.js";
import {
  validateWindowState,
  type IWindowDisplay,
} from "./windows.js";

const WINDOW_STATE_STORAGE_KEYS: Readonly<Record<WorkbenchState, string>> = {
  [WorkbenchState.EMPTY]: "windowState.empty",
  [WorkbenchState.FOLDER]: "windowState",
  [WorkbenchState.WORKSPACE]: "windowState",
};
const WINDOW_STATE_VERSION = 1;

/** Display operations needed to restore and capture window placement. */
export interface IWindowDisplayService {
  getAllDisplays(): readonly IWindowDisplay[];
  getDisplayMatching(bounds: IWindowBounds): IWindowDisplay;
}

/**
 * BrowserWindow operations used by state persistence.
 *
 * Keeping this structural contract small makes state behavior testable without
 * loading Electron in a Node.js test process.
 */
export interface IStatefulWindow {
  isFullScreen(): boolean;
  isMaximized(): boolean;
  getBounds(): IWindowBounds;
  getNormalBounds(): IWindowBounds;
  on(event: "blur" | "close", listener: () => void): void;
  removeListener(event: "blur" | "close", listener: () => void): void;
}

/** Dependencies and error reporting used by the window state owner. */
export interface IWindowsStateHandlerOptions {
  readonly stateService: IStateService;
  readonly displayService: IWindowDisplayService;
  readonly workbenchState: WorkbenchState;
  readonly onError?: (error: unknown) => void;
}

/**
 * Owns the persisted schema and lifecycle for the main Electron window state.
 */
export class WindowsStateHandler {
  readonly #stateService: IStateService;
  readonly #displayService: IWindowDisplayService;
  readonly #workbenchState: WorkbenchState;
  readonly #storageKey: string;
  readonly #onError: (error: unknown) => void;
  #lastNormalBounds: IWindowBounds | undefined;

  constructor({
    stateService,
    displayService,
    workbenchState,
    onError = () => undefined,
  }: IWindowsStateHandlerOptions) {
    this.#stateService = stateService;
    this.#displayService = displayService;
    this.#workbenchState = workbenchState;
    this.#storageKey = WINDOW_STATE_STORAGE_KEYS[workbenchState];
    this.#onError = onError;
  }

  /** Restores validated state or returns defaults when state cannot be used. */
  restoreWindowState(): IWindowState {
    const storedState = parseStoredWindowState(
      this.#stateService.getItem(this.#storageKey),
    );
    if (!storedState) {
      return defaultWindowState(this.#workbenchState);
    }

    const restoredState = validateWindowState(
      storedState,
      this.#displayService.getAllDisplays(),
      this.#workbenchState,
    );
    if (!restoredState) {
      return defaultWindowState(this.#workbenchState);
    }

    this.#lastNormalBounds = toBounds(restoredState);
    return restoredState;
  }

  /** Saves immediately on blur and before the BrowserWindow closes. */
  trackWindow(window: IStatefulWindow): IDisposable {
    const save = (): void => {
      void this.saveWindowState(window).catch(this.#onError);
    };
    window.on("blur", save);
    window.on("close", save);

    return toDisposable(() => {
      window.removeListener("blur", save);
      window.removeListener("close", save);
    });
  }

  /** Captures normal bounds and flushes the versioned state to disk. */
  async saveWindowState(window: IStatefulWindow): Promise<void> {
    const state = this.#captureWindowState(window);
    if (!state) {
      return;
    }

    this.#stateService.setItem(
      this.#storageKey,
      serializeWindowState(state),
    );
    await this.#stateService.flush();
  }

  #captureWindowState(window: IStatefulWindow): IWindowState | undefined {
    const mode = window.isFullScreen()
      ? WindowMode.Fullscreen
      : window.isMaximized()
        ? WindowMode.Maximized
        : WindowMode.Normal;
    const primaryBounds = readBounds(() =>
      mode === WindowMode.Normal
        ? window.getBounds()
        : window.getNormalBounds()
    );
    const bounds = primaryBounds ??
      readBounds(() => window.getBounds()) ??
      this.#lastNormalBounds;
    if (!bounds) {
      return undefined;
    }

    this.#lastNormalBounds = bounds;
    let displayId: number | undefined;
    if (mode === WindowMode.Fullscreen) {
      const currentBounds = readBounds(() => window.getBounds()) ?? bounds;
      displayId = this.#displayService.getDisplayMatching(currentBounds).id;
    }

    return {
      mode,
      ...bounds,
      displayId,
    };
  }
}

function serializeWindowState(state: IWindowState): unknown {
  return {
    version: WINDOW_STATE_VERSION,
    mode: state.mode,
    bounds: {
      x: state.x,
      y: state.y,
      width: state.width,
      height: state.height,
    },
    ...(state.displayId === undefined ? {} : { displayId: state.displayId }),
  };
}

function parseStoredWindowState(value: unknown): IWindowState | undefined {
  if (!isRecord(value) || value.version !== WINDOW_STATE_VERSION) {
    return undefined;
  }
  const mode = value.mode;
  const storedBounds = value.bounds;
  if (!isWindowMode(mode) || !isRecord(storedBounds)) {
    return undefined;
  }

  const bounds = readBounds(() => ({
    x: storedBounds.x,
    y: storedBounds.y,
    width: storedBounds.width,
    height: storedBounds.height,
  }));
  if (!bounds) {
    return undefined;
  }
  if (
    value.displayId !== undefined &&
    !isFiniteNumber(value.displayId)
  ) {
    return undefined;
  }

  return {
    mode,
    ...bounds,
    displayId: value.displayId,
  };
}

function toBounds(state: IWindowState): IWindowBounds | undefined {
  if (!isFiniteNumber(state.x) || !isFiniteNumber(state.y)) {
    return undefined;
  }
  return {
    x: state.x,
    y: state.y,
    width: state.width,
    height: state.height,
  };
}

function readBounds(read: () => {
  readonly x: unknown;
  readonly y: unknown;
  readonly width: unknown;
  readonly height: unknown;
}): IWindowBounds | undefined {
  try {
    const bounds = read();
    if (
      !isFiniteNumber(bounds.x) ||
      !isFiniteNumber(bounds.y) ||
      !isFiniteNumber(bounds.width) ||
      !isFiniteNumber(bounds.height) ||
      bounds.width <= 0 ||
      bounds.height <= 0
    ) {
      return undefined;
    }
    return {
      x: bounds.x,
      y: bounds.y,
      width: bounds.width,
      height: bounds.height,
    };
  } catch {
    return undefined;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isWindowMode(value: unknown): value is IWindowState["mode"] {
  return value === WindowMode.Normal ||
    value === WindowMode.Maximized ||
    value === WindowMode.Fullscreen;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
