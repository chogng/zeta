import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../src/base/common/uri.js";
import { UNKNOWN_EMPTY_WINDOW_WORKSPACE, } from "../src/platform/workspace/common/workspace.js";
import { defaultWindowState, WindowMode, } from "../src/platform/window/electron-main/window.js";
import { applyWindowState, resolveBrowserWindowOptions, validateWindowState, } from "../src/platform/windows/electron-main/windows.js";
import { WindowsStateHandler, } from "../src/platform/windows/electron-main/windowsStateHandler.js";
const primaryDisplay = {
    id: 1,
    bounds: { x: 0, y: 0, width: 1920, height: 1080 },
    workArea: { x: 0, y: 0, width: 1920, height: 1040 },
};
const folderWorkspace = Object.freeze({
    id: "folder-project",
    uri: URI.file("C:\\projects\\folder"),
});
const multiRootWorkspace = Object.freeze({
    id: "multi-root-project",
    configPath: URI.file("C:\\projects\\team.zeta-workspace"),
});
class TestStateService {
    items = new Map();
    flushCount = 0;
    getItem(key) {
        return this.items.get(key);
    }
    setItem(key, value) {
        this.items.set(key, value);
    }
    removeItem(key) {
        this.items.delete(key);
    }
    async flush() {
        this.flushCount += 1;
    }
    async close() {
        await this.flush();
    }
}
class TestWindow {
    #listeners = new Map();
    fullscreen = false;
    maximized = false;
    bounds = { x: 0, y: 0, width: 1920, height: 1040 };
    normalBounds = { x: 120, y: 80, width: 1100, height: 760 };
    isFullScreen() {
        return this.fullscreen;
    }
    isMaximized() {
        return this.maximized;
    }
    getBounds() {
        return this.bounds;
    }
    getNormalBounds() {
        return this.normalBounds;
    }
    on(event, listener) {
        let listeners = this.#listeners.get(event);
        if (!listeners) {
            listeners = new Set();
            this.#listeners.set(event, listeners);
        }
        listeners.add(listener);
    }
    removeListener(event, listener) {
        this.#listeners.get(event)?.delete(listener);
    }
    emit(event) {
        for (const listener of this.#listeners.get(event) ?? []) {
            listener();
        }
    }
}
function createHandler(stateService, workspace, backupPath) {
    return new WindowsStateHandler({
        stateService,
        workspace,
        backupPath,
        displayService: {
            getAllDisplays: () => [primaryDisplay],
            getDisplayMatching: () => primaryDisplay,
        },
    });
}
test("default window states match the VS Code workbench sizes", () => {
    assert.deepEqual(defaultWindowState(1 /* WorkbenchState.EMPTY */), {
        mode: WindowMode.Normal,
        width: 1200,
        height: 800,
    });
    assert.deepEqual(defaultWindowState(2 /* WorkbenchState.FOLDER */), {
        mode: WindowMode.Normal,
        width: 1440,
        height: 900,
    });
    assert.deepEqual(defaultWindowState(3 /* WorkbenchState.WORKSPACE */), {
        mode: WindowMode.Normal,
        width: 1440,
        height: 900,
    });
});
test("window state falls back to defaults when persisted data is invalid", () => {
    const stateService = new TestStateService();
    stateService.setItem("windowsState", {
        version: 1,
        openedWindows: [{
                folder: folderWorkspace.uri.toString(),
                uiState: {
                    mode: WindowMode.Normal,
                    bounds: { x: 0, y: 0, width: -1, height: 800 },
                },
            }],
    });
    assert.deepEqual(createHandler(stateService, folderWorkspace).restoreWindowState(), defaultWindowState(2 /* WorkbenchState.FOLDER */));
});
test("legacy per-kind window state keys are not restored", () => {
    const stateService = new TestStateService();
    stateService.setItem("windowState", {
        version: 1,
        mode: WindowMode.Normal,
        bounds: { x: 140, y: 90, width: 1440, height: 900 },
    });
    assert.deepEqual(createHandler(stateService, UNKNOWN_EMPTY_WINDOW_WORKSPACE).restoreWindowState(), defaultWindowState(1 /* WorkbenchState.EMPTY */));
});
test("window state restores exact folder and workspace records", () => {
    const stateService = new TestStateService();
    stateService.setItem("windowsState", {
        version: 1,
        lastActiveWindow: {
            uiState: {
                mode: WindowMode.Normal,
                bounds: { x: 30, y: 40, width: 1200, height: 800 },
            },
        },
        openedWindows: [
            {
                folder: folderWorkspace.uri.toString(),
                uiState: {
                    mode: WindowMode.Normal,
                    bounds: { x: 100, y: 80, width: 1100, height: 760 },
                },
            },
            {
                workspaceIdentifier: {
                    id: multiRootWorkspace.id,
                    configURIPath: multiRootWorkspace.configPath.toString(),
                },
                uiState: {
                    mode: WindowMode.Maximized,
                    bounds: { x: 140, y: 90, width: 1000, height: 700 },
                },
            },
        ],
    });
    assert.deepEqual(createHandler(stateService, folderWorkspace).restoreWindowState(), {
        mode: WindowMode.Normal,
        x: 100,
        y: 80,
        width: 1100,
        height: 760,
        displayId: undefined,
    });
    assert.deepEqual(createHandler(stateService, multiRootWorkspace).restoreWindowState(), {
        mode: WindowMode.Maximized,
        x: 140,
        y: 90,
        width: 1000,
        height: 700,
        displayId: undefined,
    });
});
test("empty windows restore by backup path", () => {
    const stateService = new TestStateService();
    stateService.setItem("windowsState", {
        version: 1,
        lastActiveWindow: {
            uiState: {
                mode: WindowMode.Normal,
                bounds: { x: 30, y: 40, width: 1200, height: 800 },
            },
        },
        openedWindows: [{
                backupPath: "C:\\backups\\empty-1",
                uiState: {
                    mode: WindowMode.Normal,
                    bounds: { x: 180, y: 120, width: 900, height: 640 },
                },
            }],
    });
    assert.deepEqual(createHandler(stateService, UNKNOWN_EMPTY_WINDOW_WORKSPACE, "C:\\backups\\empty-1").restoreWindowState(), {
        mode: WindowMode.Normal,
        x: 180,
        y: 120,
        width: 900,
        height: 640,
        displayId: undefined,
    });
});
test("first unmatched window inherits the last active window state", () => {
    const stateService = new TestStateService();
    stateService.setItem("windowsState", {
        version: 1,
        lastActiveWindow: {
            workspaceIdentifier: {
                id: "other-project",
                configURIPath: URI.file("C:\\projects\\other.zeta-workspace").toString(),
            },
            uiState: {
                mode: WindowMode.Normal,
                bounds: { x: 200, y: 100, width: 1280, height: 820 },
            },
        },
        openedWindows: [],
    });
    assert.deepEqual(createHandler(stateService, folderWorkspace).restoreWindowState(), {
        mode: WindowMode.Normal,
        x: 200,
        y: 100,
        width: 1280,
        height: 820,
        displayId: undefined,
    });
});
test("window state is adjusted to the current single display", () => {
    const state = validateWindowState({
        mode: WindowMode.Normal,
        x: 5000,
        y: 5000,
        width: 2400,
        height: 1600,
    }, [primaryDisplay], 2 /* WorkbenchState.FOLDER */);
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
    const handler = createHandler(stateService, folderWorkspace);
    const window = new TestWindow();
    window.maximized = true;
    await handler.saveWindowState(window);
    const serializedWindow = {
        folder: folderWorkspace.uri.toString(),
        uiState: {
            mode: WindowMode.Maximized,
            bounds: {
                x: 120,
                y: 80,
                width: 1100,
                height: 760,
            },
        },
    };
    assert.deepEqual(stateService.getItem("windowsState"), {
        version: 1,
        lastActiveWindow: serializedWindow,
        openedWindows: [serializedWindow],
    });
    assert.equal(stateService.flushCount, 1);
});
test("workspace windows persist their workspace identifier", async () => {
    const stateService = new TestStateService();
    const handler = createHandler(stateService, multiRootWorkspace);
    const window = new TestWindow();
    await handler.saveWindowState(window);
    const serializedWindow = {
        workspaceIdentifier: {
            id: multiRootWorkspace.id,
            configURIPath: multiRootWorkspace.configPath.toString(),
        },
        uiState: {
            mode: WindowMode.Normal,
            bounds: {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
        },
    };
    assert.deepEqual(stateService.getItem("windowsState"), {
        version: 1,
        lastActiveWindow: serializedWindow,
        openedWindows: [serializedWindow],
    });
});
test("empty windows persist their backup identity", async () => {
    const stateService = new TestStateService();
    const handler = createHandler(stateService, UNKNOWN_EMPTY_WINDOW_WORKSPACE, "C:\\backups\\empty-1");
    const window = new TestWindow();
    await handler.saveWindowState(window);
    const serializedWindow = {
        backupPath: "C:\\backups\\empty-1",
        uiState: {
            mode: WindowMode.Normal,
            bounds: {
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
            },
        },
    };
    assert.deepEqual(stateService.getItem("windowsState"), {
        version: 1,
        lastActiveWindow: serializedWindow,
        openedWindows: [serializedWindow],
    });
});
test("tracked windows save on blur and stop after disposal", async () => {
    const stateService = new TestStateService();
    const handler = createHandler(stateService, folderWorkspace);
    const window = new TestWindow();
    const tracking = handler.trackWindow(window);
    window.emit("blur");
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(stateService.flushCount, 1);
    tracking.dispose();
    window.emit("blur");
    await Promise.resolve();
    assert.equal(stateService.flushCount, 1);
});
