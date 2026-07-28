import assert from "node:assert/strict";
import test from "node:test";
import type { IDisposable } from "../src/base/common/lifecycle.js";
import type { IStateService } from "../src/platform/state/node/state.js";
import {
  WorkbenchState,
} from "../src/platform/workspace/common/workspace.js";
import {
  defaultWindowState,
  WindowMode,
  type IWindowBounds,
} from "../src/platform/window/electron-main/window.js";
import {
  applyWindowState,
  resolveBrowserWindowOptions,
  validateWindowState,
  type IWindowDisplay,
} from "../src/platform/windows/electron-main/windows.js";
import {
  WindowsStateHandler,
  type IStatefulWindow,
} from "../src/platform/windows/electron-main/windowsStateHandler.js";

const primaryDisplay: IWindowDisplay = {
  id: 1,
  bounds: { x: 0, y: 0, width: 1920, height: 1080 },
  workArea: { x: 0, y: 0, width: 1920, height: 1040 },
};

class TestStateService implements IStateService {
  readonly items = new Map<string, unknown>();
  flushCount = 0;

  getItem(key: string): unknown {
    return this.items.get(key);
  }

  setItem(key: string, value: unknown): void {
    this.items.set(key, value);
  }

  removeItem(key: string): void {
    this.items.delete(key);
  }

  async flush(): Promise<void> {
    this.flushCount += 1;
  }

  async close(): Promise<void> {
    await this.flush();
  }
}

class TestWindow implements IStatefulWindow {
  readonly #listeners = new Map<"blur" | "close", Set<() => void>>();
  fullscreen = false;
  maximized = false;
  bounds: IWindowBounds = { x: 0, y: 0, width: 1920, height: 1040 };
  normalBounds: IWindowBounds = { x: 120, y: 80, width: 1100, height: 760 };

  isFullScreen(): boolean {
    return this.fullscreen;
  }

  isMaximized(): boolean {
    return this.maximized;
  }

  getBounds(): IWindowBounds {
    return this.bounds;
  }

  getNormalBounds(): IWindowBounds {
    return this.normalBounds;
  }

  on(event: "blur" | "close", listener: () => void): void {
    let listeners = this.#listeners.get(event);
    if (!listeners) {
      listeners = new Set();
      this.#listeners.set(event, listeners);
    }
    listeners.add(listener);
  }

  removeListener(event: "blur" | "close", listener: () => void): void {
    this.#listeners.get(event)?.delete(listener);
  }

  emit(event: "blur" | "close"): void {
    for (const listener of this.#listeners.get(event) ?? []) {
      listener();
    }
  }
}

function createHandler(
  stateService: IStateService,
  workbenchState: WorkbenchState,
): WindowsStateHandler {
  return new WindowsStateHandler({
    stateService,
    workbenchState,
    displayService: {
      getAllDisplays: () => [primaryDisplay],
      getDisplayMatching: () => primaryDisplay,
    },
  });
}

test("default window states match the VS Code workbench sizes", () => {
  assert.deepEqual(defaultWindowState(WorkbenchState.EMPTY), {
    mode: WindowMode.Normal,
    width: 1200,
    height: 800,
  });
  assert.deepEqual(defaultWindowState(WorkbenchState.FOLDER), {
    mode: WindowMode.Normal,
    width: 1440,
    height: 900,
  });
});

test("window state falls back to defaults when persisted data is invalid", () => {
  const stateService = new TestStateService();
  stateService.setItem("windowState", {
    version: 1,
    mode: WindowMode.Normal,
    bounds: { x: 0, y: 0, width: -1, height: 800 },
  });

  assert.deepEqual(
    createHandler(stateService, WorkbenchState.FOLDER).restoreWindowState(),
    defaultWindowState(WorkbenchState.FOLDER),
  );
});

test("empty and workspace windows restore independent state", () => {
  const stateService = new TestStateService();
  stateService.setItem("windowState", {
    version: 1,
    mode: WindowMode.Normal,
    bounds: { x: 140, y: 90, width: 1440, height: 900 },
  });

  assert.deepEqual(
    createHandler(stateService, WorkbenchState.EMPTY).restoreWindowState(),
    defaultWindowState(WorkbenchState.EMPTY),
  );
});

test("window state restores a valid versioned payload", () => {
  const stateService = new TestStateService();
  stateService.setItem("windowState", {
    version: 1,
    mode: WindowMode.Maximized,
    bounds: { x: 140, y: 90, width: 1000, height: 700 },
  });

  assert.deepEqual(
    createHandler(stateService, WorkbenchState.WORKSPACE).restoreWindowState(),
    {
      mode: WindowMode.Maximized,
      x: 140,
      y: 90,
      width: 1000,
      height: 700,
      displayId: undefined,
    },
  );
});

test("window state is adjusted to the current single display", () => {
  const state = validateWindowState({
    mode: WindowMode.Normal,
    x: 5000,
    y: 5000,
    width: 2400,
    height: 1600,
  }, [primaryDisplay], WorkbenchState.FOLDER);

  assert.deepEqual(state, {
    mode: WindowMode.Normal,
    x: 0,
    y: 0,
    width: 1920,
    height: 1040,
  });
});

test("window options include restored bounds and defer non-normal display", () => {
  const options = resolveBrowserWindowOptions({
    state: {
      mode: WindowMode.Maximized,
      x: 100,
      y: 60,
      width: 1280,
      height: 800,
    },
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: "preload.js",
      additionalArguments: [],
    },
    platform: "linux",
  });

  assert.equal(options.show, false);
  assert.equal(options.x, 100);
  assert.equal(options.y, 60);
  assert.equal(options.width, 1280);
  assert.equal(options.height, 800);
  assert.equal(options.minWidth, 400);
  assert.equal(options.minHeight, 270);

  let maximized = false;
  applyWindowState({
    maximize: () => {
      maximized = true;
    },
    setFullScreen: () => {
      throw new Error("unexpected fullscreen");
    },
  }, { mode: WindowMode.Maximized, width: 1280, height: 800 });
  assert.equal(maximized, true);
});

test("maximized windows persist their normal bounds", async () => {
  const stateService = new TestStateService();
  const handler = createHandler(stateService, WorkbenchState.FOLDER);
  const window = new TestWindow();
  window.maximized = true;

  await handler.saveWindowState(window);

  assert.deepEqual(stateService.getItem("windowState"), {
    version: 1,
    mode: WindowMode.Maximized,
    bounds: {
      x: 120,
      y: 80,
      width: 1100,
      height: 760,
    },
  });
  assert.equal(stateService.flushCount, 1);
});

test("tracked windows save on blur and stop after disposal", async () => {
  const stateService = new TestStateService();
  const handler = createHandler(stateService, WorkbenchState.FOLDER);
  const window = new TestWindow();
  const tracking: IDisposable = handler.trackWindow(window);

  window.emit("blur");
  await Promise.resolve();
  await Promise.resolve();
  assert.equal(stateService.flushCount, 1);

  tracking.dispose();
  window.emit("blur");
  await Promise.resolve();
  assert.equal(stateService.flushCount, 1);
});
